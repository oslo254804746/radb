use radb::protocols::blocking::AdbProtocol;

#[cfg(unix)]
fn failing_status() -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(1 << 8)
}

#[cfg(windows)]
fn failing_status() -> std::process::ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(1)
}

#[test]
fn reverse_command_uses_reverse_service_and_semicolon_payload() {
    assert_eq!(
        radb::client::common::build_reverse_command("tcp:8080", "tcp:9090", false),
        "reverse:tcp:8080;tcp:9090"
    );
}

#[test]
fn forward_command_uses_forward_service_and_semicolon_payload() {
    assert_eq!(
        radb::client::common::build_forward_command("tcp:8080", "tcp:9090", true),
        "forward:norebind:tcp:8080;tcp:9090"
    );
}

#[test]
fn adb_output_failure_is_error() {
    let err = radb::client::common::adb_output_to_result(failing_status(), "", "bad").unwrap_err();
    assert!(err.to_string().contains("bad"));
}

#[test]
fn parse_device_list_preserves_serial_and_state() {
    let entries = radb::client::common::parse_device_list(
        "emulator-5554\tdevice\n192.168.0.2:5555 offline\n",
    );

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].serial, "emulator-5554");
    assert_eq!(entries[0].state.as_deref(), Some("device"));
    assert_eq!(entries[1].serial, "192.168.0.2:5555");
    assert_eq!(entries[1].state.as_deref(), Some("offline"));
}

#[test]
fn read_response_rejects_short_length_prefix() {
    let mut cursor = std::io::Cursor::new(b"00".to_vec());
    let result = AdbProtocol::read_response(&mut cursor);
    assert!(result.is_err());
}

#[test]
fn read_string_rejects_short_fixed_size_payload() {
    let mut cursor = std::io::Cursor::new(b"ab".to_vec());
    let result = AdbProtocol::read_string(&mut cursor, 4);
    assert!(result.is_err());
}

#[test]
fn encode_path_command_uses_little_endian_path_length() {
    let encoded = radb::sync_protocol::encode_path_command(radb::sync_protocol::ID_RECV, "/sdcard/a.bin");

    assert_eq!(&encoded[0..4], b"RECV");
    assert_eq!(
        u32::from_le_bytes(encoded[4..8].try_into().unwrap()),
        "/sdcard/a.bin".len() as u32
    );
    assert_eq!(&encoded[8..], b"/sdcard/a.bin");
}

#[test]
fn recv_frame_preserves_binary_data() {
    let mut frame = Vec::new();
    frame.extend_from_slice(b"DATA");
    frame.extend_from_slice(&(5_u32).to_le_bytes());
    frame.extend_from_slice(&[0x00, 0xff, b'A', b'\n', 0x80]);

    let mut cursor = std::io::Cursor::new(frame);
    let parsed = radb::sync_protocol::read_recv_frame(&mut cursor).unwrap();

    assert_eq!(
        parsed,
        radb::sync_protocol::RecvFrame::Data(vec![0x00, 0xff, b'A', b'\n', 0x80])
    );
}

#[test]
fn recv_frame_done_has_no_payload() {
    let mut frame = Vec::new();
    frame.extend_from_slice(b"DONE");
    frame.extend_from_slice(&(0_u32).to_le_bytes());

    let mut cursor = std::io::Cursor::new(frame);
    let parsed = radb::sync_protocol::read_recv_frame(&mut cursor).unwrap();

    assert_eq!(parsed, radb::sync_protocol::RecvFrame::Done);
}

#[test]
fn parse_file_info_rejects_short_buffers() {
    let err = radb::beans::parse_file_info(vec![1, 2, 3], "/tmp/a").unwrap_err();
    assert!(err.to_string().contains("short"));
}

#[cfg(feature = "blocking")]
#[test]
fn blocking_connect_reports_connection_failures() {
    match radb::AdbClient::connect("127.0.0.1:0") {
        Ok(_) => panic!("connect unexpectedly succeeded"),
        Err(err) => assert!(matches!(err, radb::AdbError::Io(_))),
    }
}
