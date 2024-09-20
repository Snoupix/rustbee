use uuid::{uuid, Uuid};

pub type MaskT = u16;

pub const HUE_BAR_1_ADDR: [u8; ADDR_LEN] = [0xE8, 0xD4, 0xEA, 0xC4, 0x62, 0x00];
pub const HUE_BAR_2_ADDR: [u8; ADDR_LEN] = [0xEC, 0x27, 0xA7, 0xD6, 0x5A, 0x9C];

// Thanks to https://gist.github.com/shinyquagsire23/f7907fdf6b470200702e75a30135caf3 for the UUIDs
pub const LIGHT_SERVICES_UUID: Uuid = uuid!("932c32bd-0000-47a2-835a-a8d455b859dd");
pub const POWER_UUID: Uuid = uuid!("932c32bd-0002-47a2-835a-a8d455b859dd");
pub const BRIGHTNESS_UUID: Uuid = uuid!("932c32bd-0003-47a2-835a-a8d455b859dd");
pub const TEMPERATURE_UUID: Uuid = uuid!("932c32bd-0004-47a2-835a-a8d455b859dd");
pub const COLOR_UUID: Uuid = uuid!("932c32bd-0005-47a2-835a-a8d455b859dd");
pub const MISC_SERVICES_UUID: Uuid = uuid!("0000180a-0000-1000-8000-00805f9b34fb");
pub const MODEL_UUID: Uuid = uuid!("00002a24-0000-1000-8000-00805f9b34fb");
pub const MANUFACTURER_UUID: Uuid = uuid!("00002a29-0000-1000-8000-00805f9b34fb");

pub const SOCKET_PATH: &str = "/var/run/rustbee-daemon.sock"; // Needs to be sudo bc /run is root owned

/// Buffer input
/// Sent by the client
/// Received by the server
pub const BUFFER_LEN: usize = ADDR_LEN + 2 + 1 + DATA_LEN; // ADDR_LEN bytes BLE UUID length + 2 for the flags (u16 divided by 2 u8)
                                                           // + 1 for the SET/GET flag + DATA_LEN for values when SET

/// Buffer output
/// Sent by the server
/// Received by the client
pub const OUTPUT_LEN: usize = 1 + 19; // 1 for output status code + 20 bytes output data (mostly because of strings)

pub const DATA_LEN: usize = 10;
pub const ADDR_LEN: usize = 6;

pub const GUI_SAVE_INTERVAL_SECS: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputCode {
    Success,
    Failure,
    DeviceNotFound,
    Streaming,
    StreamEOF,
}

impl OutputCode {
    pub fn is_success(&self) -> bool {
        matches!(*self, Self::Success)
    }
}

impl From<u8> for OutputCode {
    fn from(value: u8) -> Self {
        match value {
            0 => OutputCode::Success,
            1 => OutputCode::Failure,
            2 => OutputCode::DeviceNotFound,
            3 => OutputCode::Streaming,
            4 => OutputCode::StreamEOF,
            x => panic!("Output code is {x} which is not handled"),
        }
    }
}

impl From<OutputCode> for u8 {
    fn from(value: OutputCode) -> Self {
        match value {
            OutputCode::Success => 0,
            OutputCode::Failure => 1,
            OutputCode::DeviceNotFound => 2,
            OutputCode::Streaming => 3,
            OutputCode::StreamEOF => 4,
        }
    }
}

pub const GET: u8 = 0;
pub const SET: u8 = 1;

pub mod flags {
    use super::MaskT;

    pub const CONNECT: MaskT = 1;
    pub const DISCONNECT: MaskT = 2;
    pub const PAIR: MaskT = 3;
    pub const POWER: MaskT = 4;
    pub const COLOR_RGB: MaskT = 5;
    pub const COLOR_HEX: MaskT = 6;
    pub const COLOR_XY: MaskT = 7;
    pub const BRIGHTNESS: MaskT = 8;
    pub const NAME: MaskT = 9;
    pub const SEARCH_NAME: MaskT = 10;
}

pub mod masks {
    use super::MaskT;

    pub const CONNECT: MaskT = 1 << 0;
    pub const DISCONNECT: MaskT = 1 << 1;
    pub const PAIR: MaskT = 1 << 2;
    pub const POWER: MaskT = 1 << 3;
    pub const COLOR_RGB: MaskT = 1 << 4;
    pub const COLOR_HEX: MaskT = 1 << 5;
    pub const COLOR_XY: MaskT = 1 << 6;
    pub const BRIGHTNESS: MaskT = 1 << 7;
    pub const NAME: MaskT = 1 << 8;
    pub const SEARCH_NAME: MaskT = 1 << 9;
}
