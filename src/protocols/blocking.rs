use crate::errors::{AdbError, AdbResult};
use crate::protocols::frame;
use log::info;
use std::io::{Read, Write};

pub trait AdbProtocol: Read + Write {
    fn send_command(&mut self, command: &str) -> AdbResult<()> {
        let packet = frame::build_command_packet(command);
        self.send_all(&packet)
    }

    fn send(&mut self, data: &[u8]) -> AdbResult<usize> {
        self.send_all(data)?;
        Ok(data.len())
    }

    fn send_all(&mut self, data: &[u8]) -> AdbResult<()> {
        info!(">>>>>>> Send Size: {:#?} >>>>>>>", data.len());
        self.write_all(data)?;
        Ok(())
    }

    fn recv(&mut self, n: usize) -> AdbResult<Vec<u8>> {
        self.recv_exact(n)
    }

    fn recv_exact(&mut self, n: usize) -> AdbResult<Vec<u8>> {
        info!("<<<<<<< Try Recv Exact Size: {:#?} <<<<<<<", n);
        let mut target = vec![0; n];
        self.read_exact(&mut target)?;
        Ok(target)
    }

    fn read_string(&mut self, size: usize) -> AdbResult<String> {
        let data = self.recv_exact(size)?;
        Ok(String::from_utf8_lossy(&data).to_string())
    }

    fn read_response(&mut self) -> AdbResult<String> {
        let length_buf = self.recv_exact(4)?;
        let length = frame::parse_length_prefix(&length_buf)?;
        let data_buf = self.recv_exact(length)?;
        Ok(String::from_utf8_lossy(&data_buf).to_string())
    }

    fn read_until_close(&mut self) -> AdbResult<String> {
        let mut content = Vec::new();
        let mut buf = vec![0; 4096];
        loop {
            let bytes_read = self.read(&mut buf)?;
            if bytes_read == 0 {
                break;
            }
            content.extend_from_slice(&buf[..bytes_read]);
        }
        Ok(String::from_utf8_lossy(&content).to_string())
    }

    fn send_cmd_then_check_okay(&mut self, command: &str) -> AdbResult<()> {
        self.send_command(command)?;
        let response = self.recv_exact(4)?;

        if frame::is_okay_response(&response) {
            Ok(())
        } else if frame::is_fail_response(&response) {
            let error_msg = self.read_response()?;
            Err(AdbError::command_failed(command, error_msg))
        } else {
            Err(AdbError::protocol_error("Unexpected response"))
        }
    }
}

impl<T: Read + Write> AdbProtocol for T {}
