use tokio::fs;
use uuid::{uuid, Uuid};

pub type MaskT = u8;

pub const HUE_BAR_1_ADDR: [u8; 6] = [0xE8, 0xD4, 0xEA, 0xC4, 0x62, 0x00];
pub const HUE_BAR_2_ADDR: [u8; 6] = [0xEC, 0x27, 0xA7, 0xD6, 0x5A, 0x9C];

// Thanks to https://gist.github.com/shinyquagsire23/f7907fdf6b470200702e75a30135caf3 for the UUIDs
pub const LIGHT_SERVICES: Uuid = uuid!("932c32bd-0000-47a2-835a-a8d455b859dd");
pub const POWER: Uuid = uuid!("932c32bd-0002-47a2-835a-a8d455b859dd");
pub const BRIGHTNESS: Uuid = uuid!("932c32bd-0003-47a2-835a-a8d455b859dd");
pub const TEMPERATURE: Uuid = uuid!("932c32bd-0004-47a2-835a-a8d455b859dd");
pub const COLOR: Uuid = uuid!("932c32bd-0005-47a2-835a-a8d455b859dd");

pub const RUN_PATH: &str = "/var/run/bluedaemon.sock"; // Needs to be sudo bc /run is root owned

pub const BUFFER_LEN: usize = 6 + 1 + 1 + DATA_LEN; // 6 bytes BLE id length + 1 for the flags
                                                    // + 1 for the SET/GET flag + 6 for values when SET
pub const DATA_LEN: usize = 6;
pub const OUTPUT_LEN: usize = 5; // 4 bytes output data + 1 for status which is SUCCESS or FAILURE

pub const SUCCESS: MaskT = 1;
pub const FAILURE: MaskT = 0;

pub const SET: MaskT = 1;
pub const GET: MaskT = 0;

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
}

pub async fn get_path() -> &'static str {
    if fs::read_dir("/var/run").await.is_err() {
        eprintln!("Cannot find /var/run directory or lacking permissions to read it");
        std::process::exit(2);
    }

    if fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open("/var/run/x")
        .await
        .is_err()
    {
        eprintln!("Lacking permissions to write to /var/run directory");
        std::process::exit(2);
    }

    let _ = fs::remove_file("/var/run/x").await;

    RUN_PATH
}
