use uuid::{uuid, Uuid};

pub type MaskT = u16;

pub const HUE_BAR_1_ADDR: [u8; 6] = [0xE8, 0xD4, 0xEA, 0xC4, 0x62, 0x00];
pub const HUE_BAR_2_ADDR: [u8; 6] = [0xEC, 0x27, 0xA7, 0xD6, 0x5A, 0x9C];

// Thanks to https://gist.github.com/shinyquagsire23/f7907fdf6b470200702e75a30135caf3 for the UUIDs
pub const LIGHT_SERVICES_UUID: Uuid = uuid!("932c32bd-0000-47a2-835a-a8d455b859dd");
pub const POWER_UUID: Uuid = uuid!("932c32bd-0002-47a2-835a-a8d455b859dd");
pub const BRIGHTNESS_UUID: Uuid = uuid!("932c32bd-0003-47a2-835a-a8d455b859dd");
pub const TEMPERATURE_UUID: Uuid = uuid!("932c32bd-0004-47a2-835a-a8d455b859dd");
pub const COLOR_UUID: Uuid = uuid!("932c32bd-0005-47a2-835a-a8d455b859dd");

pub const SOCKET_PATH: &str = "/var/run/rustbee-daemon.sock"; // Needs to be sudo bc /run is root owned

pub const BUFFER_LEN: usize = 6 + 2 + 1 + DATA_LEN; // 6 bytes BLE id length + 2 for the flags
                                                    // + 1 for the SET/GET flag + 6 for values when SET
pub const DATA_LEN: usize = 6;
pub const OUTPUT_LEN: usize = 13; // 8 bytes output data + 1 for status which is SUCCESS or FAILURE
                                  // (initially 4 bytes but extented to contain a few string bytes)

pub const SUCCESS: u8 = 1;
pub const FAILURE: u8 = 0;

pub const SET: u8 = 1;
pub const GET: u8 = 0;

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
}

pub async fn get_path() -> &'static str {
    SOCKET_PATH
}