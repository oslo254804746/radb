use crate::errors::{AdbError, AdbResult};

pub fn build_command_packet(command: &str) -> Vec<u8> {
    let cmd_bytes = command.as_bytes();
    let length = format!("{:04x}", cmd_bytes.len());
    let mut packet = Vec::with_capacity(4 + cmd_bytes.len());
    packet.extend_from_slice(length.as_bytes());
    packet.extend_from_slice(cmd_bytes);
    packet
}

pub fn parse_length_prefix(data: &[u8]) -> AdbResult<usize> {
    if data.len() != 4 {
        return Err(AdbError::protocol_error("Invalid length prefix"));
    }

    let length_str = std::str::from_utf8(data)?;
    usize::from_str_radix(length_str, 16)
        .map_err(|_| AdbError::protocol_error("Invalid length prefix"))
}

pub fn is_okay_response(data: &[u8]) -> bool {
    data == b"OKAY"
}

pub fn is_fail_response(data: &[u8]) -> bool {
    data == b"FAIL"
}
