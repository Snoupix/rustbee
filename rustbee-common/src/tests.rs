use crate::constants::{OutputCode, HUE_BAR_1_ADDR};
use crate::utils::{addr_to_uint, uint_to_addr};

#[test]
fn output_codes_consistency() {
    assert_eq!(u8::from(OutputCode::Success), 0);
    assert!(matches!(OutputCode::from(0), OutputCode::Success));

    assert_eq!(u8::from(OutputCode::Failure), 1);
    assert!(matches!(OutputCode::from(1), OutputCode::Failure));

    assert_eq!(u8::from(OutputCode::DeviceNotFound), 2);
    assert!(matches!(OutputCode::from(2), OutputCode::DeviceNotFound));

    assert_eq!(u8::from(OutputCode::Streaming), 3);
    assert!(matches!(OutputCode::from(3), OutputCode::Streaming));

    assert_eq!(u8::from(OutputCode::StreamEOF), 4);
    assert!(matches!(OutputCode::from(4), OutputCode::StreamEOF));
}

#[test]
fn uint_conversion() {
    let addr = HUE_BAR_1_ADDR;
    let uint = uint_to_addr(0xE8D4EAC46200);
    assert_eq!(addr, uint);
}

#[test]
fn address_conversion() {
    let uint = 0xE8D4EAC46200;
    let addr = addr_to_uint(&HUE_BAR_1_ADDR);
    assert_eq!(addr, uint);
}
