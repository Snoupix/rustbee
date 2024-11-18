// Re-exports
pub use super::daemon::*;

use crate::constants::ADDR_LEN;

pub fn addr_to_uint(addr: &[u8; ADDR_LEN]) -> u64 {
    let mut res: u64 = 0;

    for byte in addr {
        res = (res << 8) | (*byte as u64);
    }

    res
}

pub fn uint_to_addr(addr: u64) -> [u8; ADDR_LEN] {
    let mut res = [0; ADDR_LEN];

    for i in 0..res.len() {
        res[res.len() - 1 - i] = ((addr >> (i * 8)) & 0xff) as _;
    }

    res
}
