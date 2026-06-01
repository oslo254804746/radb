use crate::errors::{AdbError, AdbResult};
use crate::protocols::frame;
use async_trait::async_trait;
use log::info;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[async_trait]
pub trait AdbProtocol: AsyncRead + AsyncWrite + Unpin + Send {
    async fn send_command(&mut self, command: &str) -> AdbResult<()> {
        let packet = frame::build_command_packet(command);
        self.send_all(&packet).await
    }

    async fn send(&mut self, data: &[u8]) -> AdbResult<usize> {
        self.send_all(data).await?;
        Ok(data.len())
    }

    async fn send_all(&mut self, data: &[u8]) -> AdbResult<()> {
        info!(">>>>>>> Send Size: {:#?} >>>>>>>", data.len());
        self.write_all(data).await?;
        Ok(())
    }

    async fn recv(&mut self, n: usize) -> AdbResult<Vec<u8>> {
        self.recv_exact(n).await
    }

    async fn recv_exact(&mut self, n: usize) -> AdbResult<Vec<u8>> {
        info!("<<<<<<< Try Recv Exact Size: {:#?} <<<<<<<", n);
        let mut target = vec![0; n];
        self.read_exact(&mut target).await?;
        Ok(target)
    }

    async fn read_string(&mut self, size: usize) -> AdbResult<String> {
        let data = self.recv_exact(size).await?;
        Ok(String::from_utf8_lossy(&data).to_string())
    }

    async fn read_response(&mut self) -> AdbResult<String> {
        let length_buf = self.recv_exact(4).await?;
        let length = frame::parse_length_prefix(&length_buf)?;
        let data_buf = self.recv_exact(length).await?;
        Ok(String::from_utf8_lossy(&data_buf).to_string())
    }

    async fn send_cmd_then_check_okay(&mut self, command: &str) -> AdbResult<()> {
        self.send_command(command).await?;
        let response = self.recv_exact(4).await?;

        if frame::is_okay_response(&response) {
            Ok(())
        } else if frame::is_fail_response(&response) {
            let error_msg = self.read_response().await?;
            Err(AdbError::command_failed(command, error_msg))
        } else {
            Err(AdbError::protocol_error("Unexpected response"))
        }
    }

    async fn read_until_close(&mut self) -> AdbResult<String> {
        let mut content = Vec::new();
        let mut buf = vec![0; 4096];
        loop {
            let bytes_read = self.read(&mut buf).await?;
            if bytes_read == 0 {
                break;
            }
            content.extend_from_slice(&buf[..bytes_read]);
        }
        Ok(String::from_utf8_lossy(&content).to_string())
    }
}

#[async_trait]
impl<T> AdbProtocol for T where T: AsyncRead + AsyncWrite + Unpin + Send {}
