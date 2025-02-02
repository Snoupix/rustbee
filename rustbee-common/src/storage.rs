use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use log::*;

use crate::constants::{ADDR_LEN, APP_ID};

type Data = HashMap<[u8; ADDR_LEN], SavedDevice>;

#[derive(Clone)]
pub struct Storage {
    path: PathBuf,
    data: Data,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SavedDevice {
    pub name: String,
    pub current_color: [u8; 3],
    pub brightness: u8,
}

impl Storage {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            data: HashMap::new(),
        }
    }

    pub fn try_default() -> Result<Self, String> {
        // yes, eframe is imported only for that :clown:
        // TODO: Impl cross-platform storage_dir
        let path = eframe::storage_dir(APP_ID);

        if path.is_none() {
            return Err("Cannot get default eframe::storage_dir, please use Storage::new and specify the path".into());
        }

        let path = path.unwrap();

        #[cfg(target_os = "windows")]
        {
            let mut path = path.clone();
            path.pop();
            let exists = std::fs::exists(path.clone());
            if exists.is_err() || !exists.unwrap() {
                if let Err(err) = std::fs::create_dir(path.clone()) {
                    return Err(format!(
                        "Failed to create storage dir at {} ({err})",
                        path.display()
                    ));
                }
            }
        }

        Ok(Self {
            path,
            data: HashMap::new(),
        })
    }

    fn serialize_data(&self) -> HashMap<String, SavedDevice> {
        self.data
            .iter()
            .map(|(addr, device)| {
                let addr = addr
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<Vec<_>>()
                    .join(":");

                (addr, device.clone())
            })
            .collect()
    }

    fn deserialize_data(&self, data: HashMap<String, SavedDevice>) -> Data {
        data.into_iter()
            .map(|(addr, device)| (parse_hex_address(&addr), device))
            .collect()
    }

    fn load_from_file(&mut self) {
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(err) => {
                if !matches!(err.kind(), std::io::ErrorKind::NotFound) {
                    panic!("Failed to open saved data file in read-only {err}");
                }
                return;
            }
        };

        let mut content = String::new();
        file.read_to_string(&mut content)
            .expect("Failed to read from storage file");

        match serde_json::from_str::<HashMap<String, SavedDevice>>(&content) {
            Ok(data) => self.data = self.deserialize_data(data),
            Err(err) => error!("Failed to deserialize saved data {err}"),
        }
    }

    pub fn get_device(&mut self, addr: &[u8; ADDR_LEN]) -> Option<&SavedDevice> {
        if self.data.is_empty() {
            self.load_from_file();
        }

        self.data.get(addr)
    }

    pub fn get_devices(&mut self) -> &Data {
        if self.data.is_empty() {
            self.load_from_file();
        }

        &self.data
    }

    pub fn set_device(&mut self, addr: [u8; ADDR_LEN], device: Option<SavedDevice>) {
        self.data.insert(addr, device.unwrap_or_default());
    }

    pub fn set_devices(&mut self, devices: Vec<([u8; ADDR_LEN], Option<SavedDevice>)>) {
        for (addr, device) in devices {
            self.data.insert(addr, device.unwrap_or_default());
        }
    }

    /// Save to disk
    pub fn flush(&self) {
        let mut file = if !fs::exists(&self.path).unwrap() {
            File::create(&self.path).expect("Failed to create storage file")
        } else {
            File::options()
                .write(true)
                .open(&self.path)
                .expect("Failed to open storage file in write-only")
        };

        if let Err(err) = file.write_all(
            serde_json::to_string(&self.serialize_data())
                .expect("Cannot parse storage data to String")
                .as_bytes(),
        ) {
            error!("Failed to write to storage file data {err}");
            return;
        }

        file.flush().expect("Failed to write to storage file");
    }
}

fn parse_hex_address(address: &str) -> [u8; ADDR_LEN] {
    let mut addr = [0; ADDR_LEN];
    let chars = address.chars().filter(|c| *c != ':');
    let bytes = chars
        .clone()
        .step_by(2)
        .zip(chars.skip(1).step_by(2))
        .map(|(a, b)| {
            u8::from_str_radix(&format!("{a}{b}"), 16)
                .map_err(|e| panic!("[ERROR] Cannot parse {address} to hex value {e}"))
                .unwrap()
        })
        .collect::<Vec<_>>();

    assert!(
        bytes.len() == ADDR_LEN,
        "[ERROR] Hex address {address} is not right. It must be of length {ADDR_LEN}"
    );

    for (i, byte) in bytes.into_iter().enumerate() {
        addr[i] = byte;
    }

    addr
}
