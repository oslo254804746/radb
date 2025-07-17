#[cfg(feature = "blocking")]
pub use blocking::AdbProtocol;

#[cfg(feature = "tokio_async")]
pub use tokio_async::AdbProtocol;

pub mod protocol_logic {
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
        if data.len() < 4 {
            return Err(AdbError::protocol_error("Invalid length prefix"));
        }
        let length_str = String::from_utf8_lossy(&data[..4]);
        usize::from_str_radix(&length_str, 16)
            .map_err(|_| AdbError::protocol_error("Invalid length "))
    }

    pub fn is_okay_response(data: &[u8]) -> bool {
        data == b"OKAY"
    }

    pub fn is_fail_response(data: &[u8]) -> bool {
        data == b"FAIL"
    }
}

#[cfg(feature = "blocking")]
pub mod blocking {
    use super::protocol_logic;
    use crate::errors::{AdbError, AdbResult as Result, AdbResult};
    use log::info;
    pub trait AdbProtocol: std::io::Read + std::io::Write {
        fn send_command(&mut self, command: &str) -> Result<()> {
            let packet = protocol_logic::build_command_packet(command);
            self.write_all(&packet)?;
            Ok(())
        }

        fn send(&mut self, data: &[u8]) -> Result<usize> {
            info!(">>>>>>> Send Size: {:#?} >>>>>>>", data.len());
            let size = self.write(data)?;
            Ok(size)
        }

        fn recv(&mut self, n: usize) -> Result<Vec<u8>> {
            info!("<<<<<<< Try Recv Size: {:#?} <<<<<<<", n);
            let mut target = vec![0; n];
            let result = self.read(&mut target)?;
            info!("<<<<<<< Recv Size: {:#?} <<<<<<<", result);
            Ok(target[..result].to_owned())
        }

        fn read_string(&mut self, size: usize) -> AdbResult<String> {
            let data = self.recv(size)?;
            let resp = String::from_utf8_lossy(&data).to_string();
            Ok(resp)
        }

        fn read_response(&mut self) -> Result<String> {
            let length_buf = self.recv(4)?;
            let length = protocol_logic::parse_length_prefix(&length_buf).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid length")
            })?;

            let mut data_buf = vec![0; length];
            self.read_exact(&mut data_buf)?;
            Ok(String::from_utf8_lossy(&data_buf).to_string())
        }
        fn read_until_close(&mut self) -> Result<String> {
            let mut content = Vec::new();
            loop {
                let bytes_read = self.recv(4096)?;
                if bytes_read.is_empty() {
                    break;
                }
                content.extend_from_slice(&bytes_read);
            }
            Ok(String::from_utf8_lossy(&content).to_string())
        }

        fn send_cmd_then_check_okay(&mut self, command: &str) -> Result<()> {
            self.send_command(command)?;
            let mut response = [0; 4];
            self.read_exact(&mut response)?;

            if protocol_logic::is_okay_response(&response) {
                Ok(())
            } else if protocol_logic::is_fail_response(&response) {
                let error_msg = self.read_response()?;
                Err(AdbError::network_error(error_msg))
            } else {
                Err(AdbError::parse_error("Unexpected response"))
            }
        }
    }

    impl<T: std::io::Read + std::io::Write> AdbProtocol for T {}
}

#[cfg(feature = "tokio_async")]
pub mod tokio_async {
    use super::protocol_logic;
    use crate::errors::{AdbError, AdbResult};
    use async_trait::async_trait;
    use log::info;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[async_trait]
    pub trait AdbProtocol: AsyncReadExt + AsyncWriteExt + Unpin {
        async fn send(&mut self, data: &[u8]) -> AdbResult<usize> {
            info!(">>>>>>> Send Size: {:#?} >>>>>>>", data.len());
            let size = self.write(data).await?;
            Ok(size)
        }
        async fn send_command(&mut self, command: &str) -> AdbResult<()> {
            let packet = protocol_logic::build_command_packet(command);
            self.write_all(&packet).await?;
            Ok(())
        }
        async fn recv(&mut self, n: usize) -> AdbResult<Vec<u8>> {
            info!("<<<<<<< Try Recv Size: {:#?} <<<<<<<", n);
            let mut target = vec![0; n];
            let result = self.read(&mut target).await?;
            info!("<<<<<<< Recv Size: {:#?} <<<<<<<", result);
            Ok(target[..result].to_owned())
        }

        async fn read_string(&mut self, size: usize) -> AdbResult<String> {
            let mut obj = vec![0; size]; // 有问题
            let data = self.read(&mut obj).await?;
            let resp = String::from_utf8_lossy(&obj).to_string();
            Ok(resp)
        }
        async fn read_response(&mut self) -> std::io::Result<String> {
            let mut length_buf = [0; 4];
            self.read_exact(&mut length_buf).await?;

            let length = protocol_logic::parse_length_prefix(&length_buf).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid length")
            })?;

            let mut data_buf = vec![0; length];
            self.read_exact(&mut data_buf).await?;
            Ok(String::from_utf8_lossy(&data_buf).to_string())
        }

        async fn send_cmd_then_check_okay(&mut self, command: &str) -> AdbResult<()> {
            self.send_command(command).await?;
            let mut response = [0; 4];
            self.read_exact(&mut response).await?;

            if protocol_logic::is_okay_response(&response) {
                Ok(())
            } else if protocol_logic::is_fail_response(&response) {
                let error_msg = self.read_response().await?;
                Err(AdbError::command_failed(command, error_msg))
            } else {
                Err(AdbError::command_failed(command, "Unexpected response"))
            }
        }

        async fn read_until_close(&mut self) -> AdbResult<String> {
            let mut content = Vec::new();
            loop {
                let bytes_read = self.recv(4096).await?;
                if bytes_read.is_empty() {
                    break;
                }
                content.extend_from_slice(&bytes_read);
            }
            Ok(String::from_utf8_lossy(&content).to_string())
        }
    }

    #[async_trait]
    impl<T: AsyncReadExt + AsyncWriteExt + Unpin + Send> AdbProtocol for T {}
}
