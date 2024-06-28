// At the beginning the CLI and the daemon were seperated crates and I know this could be a module
// now but what would life be without a bit of fun ?!
use tokio::fs;

const RUN_PATH: &str = "/var/run/bluedaemon.sock"; // Needs to be sudo bc /run is root owned

pub const SUCCESS: MaskT = 1 << 0;

const CONNECT: MaskT = 1 << 0;
const DISCONNECT: MaskT = 1 << 1;
// const DEVICE_VALUE: std::ops::Range<MaskT> = 1 << 2..1 << ((6 * 8) + 2); // 6 * 8bits (HEX value of BLE ids) + start offset

type MaskT = u8;

struct Mask(MaskT);

impl TryFrom<MaskT> for Mask {
    type Error = &'static str;

    fn try_from(value: MaskT) -> Result<Self, Self::Error> {
        if 0b11 & value == 0b11 {
            return Err("Cannot connect AND disconnect at the same time");
        }

        Ok(Self(value))
    }
}

impl std::ops::Deref for Mask {
    type Target = MaskT;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

async fn get_path() -> &'static str {
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
