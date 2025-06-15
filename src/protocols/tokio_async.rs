use anyhow::{anyhow, Context};

use log::info;

#[cfg(feature = "tokio_async")]
use async_trait::async_trait;
#[cfg(feature = "tokio_async")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::protocols::AdbProtocolRespDataType;

#[cfg(feature = "tokio_async")]
#[async_trait]
pub trait AdbProtocol: AsyncReadExt + AsyncWriteExt + Unpin {
    async fn send(&mut self, data: &[u8]) -> anyhow::Result<usize> {
        info!(">>>>>>> Send Size: {:#?} >>>>>>>", data.len());
        let size = self.write(data).await?;
        Ok(size)
    }

    /// 从设备接收指定数量的数据。
    ///
    /// # 参数
    /// - `n`: 要接收的数据大小。
    ///
    /// # 返回值
    /// - 成功返回接收到的数据，失败返回错误。
    async fn recv(&mut self, n: usize) -> anyhow::Result<Vec<u8>> {
        info!("<<<<<<< Try Recv Size: {:#?} <<<<<<<", n);
        let mut target = vec![0; n];
        let result = self.read(&mut target).await?;
        info!("<<<<<<< Recv Size: {:#?} <<<<<<<", result);
        Ok(target[..result].to_owned())
    }

    /// 发送命令到设备。
    ///
    /// # 参数
    /// - `command`: 要发送的命令字符串。
    ///
    /// # 返回值
    /// - 成功返回发送的数据大小，失败返回错误。
    async fn send_command(&mut self, command: &str) -> anyhow::Result<usize> {
        info!("Send COMMAND: <{:#?}>", command);
        let cmd_bytes = command.as_bytes();
        let length = format!("{:04x}", cmd_bytes.len());
        let mut data = Vec::with_capacity(length.len() + cmd_bytes.len());
        data.extend_from_slice(length.as_bytes());
        data.extend_from_slice(cmd_bytes);
        self.send(&data).await
    }

    /// 读取指定大小的字符串。
    ///
    /// # 参数
    /// - `size`: 字符串的字节大小。
    ///
    /// # 返回值
    /// - 成功返回读取的字符串，失败返回错误。
    async fn read_string(&mut self, size: usize) -> anyhow::Result<String> {
        let data = self.recv(size).await?;
        Ok(String::from_utf8_lossy(&data).to_string())
    }

    /// 读取一个字符串块，以字符串长度开始。
    ///
    /// # 返回值
    /// - 成功返回读取的字符串，失败返回错误。
    async fn read_string_block(&mut self) -> anyhow::Result<String> {
        let string_length = self.read_string(4).await?;
        let string_size =
            usize::from_str_radix(&string_length, 16).context("Failed to parse string length")?;
        self.read_string(string_size).await
    }

    /// 读取直到关闭的消息。
    ///
    /// # 返回值
    /// - 成功返回读取的全部内容，失败返回错误。
    async fn read_until_close(&mut self) -> anyhow::Result<String> {
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

    /// 检查设备返回是否为"OKAY"。
    ///
    /// # 返回值
    /// - 成功返回()`，表示检查通过，失败返回错误。
    async fn check_okay(&mut self) -> anyhow::Result<()> {
        let data = self.read_string(4).await?;
        info!("Check Okay Response >>> {:#?}", &data);
        if data.eq(AdbProtocolRespDataType::OKAY.as_str()) {
            return Ok(());
        }
        Err(anyhow!("Check Okay Failed"))
    }

    async fn send_cmd_then_check_okay(&mut self, command: &str) -> anyhow::Result<()> {
        self.send_command(command).await?;
        self.check_okay().await?;
        Ok(())
    }
}

#[cfg(feature = "tokio_async")]
#[async_trait]
impl<T> AdbProtocol for T where T: AsyncReadExt + AsyncWriteExt + Unpin {}
