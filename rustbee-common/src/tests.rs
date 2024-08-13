use crate::constants::OutputCode;

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
