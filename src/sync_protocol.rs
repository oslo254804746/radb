use crate::errors::{AdbError, AdbResult};
use std::io::Read;

pub const ID_STAT: &[u8; 4] = b"STAT";
pub const ID_LIST: &[u8; 4] = b"LIST";
pub const ID_RECV: &[u8; 4] = b"RECV";
pub const ID_DATA: &[u8; 4] = b"DATA";
pub const ID_DONE: &[u8; 4] = b"DONE";
pub const ID_FAIL: &[u8; 4] = b"FAIL";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecvFrame {
    Data(Vec<u8>),
    Done,
    Fail(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvFrameHeader {
    Payload { id: [u8; 4], payload_len: usize },
    Done,
}

pub fn encode_path_command(command: &[u8; 4], path: &str) -> Vec<u8> {
    let path_bytes = path.as_bytes();
    let mut out = Vec::with_capacity(8 + path_bytes.len());
    out.extend_from_slice(command);
    out.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(path_bytes);
    out
}

pub fn parse_u32_le(data: &[u8]) -> AdbResult<u32> {
    if data.len() != 4 {
        return Err(AdbError::protocol_error("Invalid little-endian u32"));
    }
    Ok(u32::from_le_bytes(data.try_into().map_err(|_| {
        AdbError::protocol_error("Invalid little-endian u32")
    })?))
}

pub fn parse_recv_frame_header(id: [u8; 4], length: [u8; 4]) -> AdbResult<RecvFrameHeader> {
    match &id {
        ID_DATA | ID_FAIL => Ok(RecvFrameHeader::Payload {
            id,
            payload_len: u32::from_le_bytes(length) as usize,
        }),
        ID_DONE => Ok(RecvFrameHeader::Done),
        _ => Err(AdbError::protocol_error(format!(
            "Unexpected sync frame id: {}",
            String::from_utf8_lossy(&id)
        ))),
    }
}

pub fn parse_recv_frame(id: [u8; 4], payload: Vec<u8>) -> AdbResult<RecvFrame> {
    match &id {
        ID_DATA => Ok(RecvFrame::Data(payload)),
        ID_DONE => {
            if payload.is_empty() {
                Ok(RecvFrame::Done)
            } else {
                Err(AdbError::protocol_error("DONE frame must not have payload"))
            }
        }
        ID_FAIL => Ok(RecvFrame::Fail(
            String::from_utf8_lossy(&payload).to_string(),
        )),
        _ => Err(AdbError::protocol_error(format!(
            "Unexpected sync frame id: {}",
            String::from_utf8_lossy(&id)
        ))),
    }
}

pub fn read_recv_frame<R: Read>(reader: &mut R) -> AdbResult<RecvFrame> {
    let mut id = [0_u8; 4];
    reader.read_exact(&mut id)?;

    let mut len = [0_u8; 4];
    reader.read_exact(&mut len)?;
    let header = parse_recv_frame_header(id, len)?;

    let RecvFrameHeader::Payload { id, payload_len } = header else {
        return Ok(RecvFrame::Done);
    };

    let mut payload = vec![0_u8; payload_len];
    if payload_len > 0 {
        reader.read_exact(&mut payload)?;
    }

    parse_recv_frame(id, payload)
}
