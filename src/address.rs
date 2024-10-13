use rustbee_common::constants::ADDR_LEN;
use rustbee_common::storage::Storage;

pub fn save_addresses(storage: &mut Storage, addresses: &[[u8; ADDR_LEN]]) {
    storage.set_devices(addresses.iter().map(|addr| (*addr, None)).collect());

    storage.flush()
}

pub fn parse_hex_address(address: &str) -> [u8; ADDR_LEN] {
    let mut addr = [0; ADDR_LEN];
    let chars = address.chars().filter(|c| *c != ':');
    let bytes = chars
        .clone()
        .step_by(2)
        .zip(chars.skip(1).step_by(2))
        .map(|(a, b)| {
            u8::from_str_radix(&format!("{a}{b}"), 16)
                .map_err(|e| {
                    panic!("[ERROR] Cannot parse {address} to hex value, try xx:xx:xx... {e}")
                })
                .unwrap()
        })
        .collect::<Vec<_>>();

    assert!(
        bytes.len() == ADDR_LEN,
        "[ERROR] Hex address {address} is not right. It must be of length {ADDR_LEN} => xx:xx:xx:xx:xx:xx"
    );

    for (i, byte) in bytes.into_iter().enumerate() {
        addr[i] = byte;
    }

    addr
}
