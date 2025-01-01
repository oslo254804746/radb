use crate::client::adb_device::AdbDevice;

use anyhow::{anyhow, Context, Result};

#[cfg(feature = "tokio_async")]
use futures_core::Stream;
#[cfg(feature = "tokio_async")]
use futures_util::stream;
#[cfg(feature = "tokio_async")]
use tokio::net::{TcpStream, ToSocketAddrs};

use crate::protocols::AdbProtocol;
#[cfg(feature = "blocking")]
use std::net::{TcpStream, ToSocketAddrs};

pub struct AdbClient {
    stream: TcpStream,
}

impl AdbClient {
    pub fn parse_device_list_lines<T>(
        lines: &str,
        addr: T,
    ) -> Result<Vec<AdbDevice<impl ToSocketAddrs + Clone>>>
    where
        T: ToSocketAddrs + Clone,
    {
        let mut devices = vec![];
        if !lines.is_empty() {
            lines.lines().into_iter().for_each(|line| {
                let parts: Vec<&str> = line.split("\t").collect();
                if !parts.is_empty() {
                    let device = AdbDevice::new(parts[0], addr.clone());
                    devices.push(device)
                }
            })
        };
        Ok(devices)
    }
}

#[cfg(feature = "tokio_async")]
impl AdbClient {
    pub async fn new<T>(addr: T) -> Self
    where
        T: ToSocketAddrs,
    {
        let stream = TcpStream::connect(addr).await.unwrap();
        Self { stream }
    }

    /// 以迭代器的形式列出所有连接的 ADB 设备。
    ///
    /// # 返回值
    /// 返回一个设备迭代器，如果获取设备列表失败，则返回错误。
    pub async fn iter_devices(
        &mut self,
    ) -> impl Stream<Item = AdbDevice<impl ToSocketAddrs + Clone>> {
        let devices = self
            .list_devices()
            .await
            .map_err(|e| anyhow!("Get Device List Error {}", e))
            .unwrap();
        stream::iter(devices)
    }

    /// 获取 ADB 服务器的版本号。
    ///
    /// # 返回值
    /// 返回服务器的版本号字符串，如果获取失败，则返回错误。
    pub async fn server_version(&mut self) -> Result<String> {
        let command = "host:version";
        self.stream.send_cmd_then_check_okay(command).await?;
        let version_string = self.stream.read_string_block().await?;
        let version = usize::from_str_radix(&version_string, 16)?;
        Ok(version.to_string())
    }

    /// 关闭 ADB 服务器。
    ///
    /// # 返回值
    /// 如果关闭成功，则返回空结果，否则返回错误。
    pub async fn server_kill(&mut self) -> Result<()> {
        let command = "host:kill";
        self.stream.send_cmd_then_check_okay(command).await?;
        Ok(())
    }

    /// 连接到指定的 ADB 设备。
    ///
    /// # 参数
    /// - `serial`: 设备的序列号，用于指定要连接的设备。
    ///
    /// # 返回值
    /// 返回连接结果的字符串表示，如果连接失败，则返回错误。
    pub async fn connect_device(&mut self, serial: &str) -> Result<String> {
        let command = format!("host:connect:{}", serial);
        self.stream.send_cmd_then_check_okay(&command).await?;
        let result = self.stream.read_string_block().await?;
        Ok(result)
    }

    /// 断开与指定 ADB 设备的连接。
    ///
    /// # 参数
    /// - `serial`: 设备的序列号，用于指定要断开连接的设备。
    ///
    /// # 返回值
    /// 返回断开连接结果的字符串表示，如果断开连接失败，则返回错误。
    pub async fn disconnect_device(&mut self, serial: &str) -> Result<String> {
        if serial.is_empty() {
            return Err(anyhow!("serial is empty"));
        }
        let command = format!("host:disconnect:{}", serial);
        self.stream.send_cmd_then_check_okay(&command).await?;
        Ok(self.stream.read_string_block().await?)
    }

    pub async fn list_devices(&mut self) -> Result<Vec<AdbDevice<impl ToSocketAddrs + Clone>>> {
        self.stream.send_cmd_then_check_okay("host:devices").await?;
        let resp = self.stream.read_string_block().await?;
        Self::parse_device_list_lines(&resp, self.stream.local_addr()?.clone())
    }
}

#[cfg(feature = "blocking")]
impl AdbClient {
    pub fn new<T>(addr: T) -> Self
    where
        T: ToSocketAddrs,
    {
        let stream = TcpStream::connect(addr).unwrap();
        Self { stream }
    }

    /// 以迭代器的形式列出所有连接的 ADB 设备。
    ///
    /// # 返回值
    /// 返回一个设备迭代器，如果获取设备列表失败，则返回错误。
    pub fn iter_devices(
        &mut self,
    ) -> Result<impl Iterator<Item = AdbDevice<impl ToSocketAddrs + Clone>>> {
        Ok(self
            .list_devices()
            .context("Get Device List Error")?
            .into_iter())
    }

    pub fn list_devices(&mut self) -> Result<Vec<AdbDevice<impl ToSocketAddrs + Clone>>> {
        self.stream.send_cmd_then_check_okay("host:devices")?;
        let resp = self.stream.read_string_block()?;
        Self::parse_device_list_lines(&resp, self.stream.local_addr()?.clone())
    }

    /// 获取 ADB 服务器的版本号。
    ///
    /// # 返回值
    /// 返回服务器的版本号字符串，如果获取失败，则返回错误。
    pub fn server_version(&mut self) -> Result<String> {
        let command = "host:version";
        self.stream.send_cmd_then_check_okay(command)?;
        let version_string = self.stream.read_string_block()?;
        let version = usize::from_str_radix(&version_string, 16)?;
        Ok(version.to_string())
    }

    /// 关闭 ADB 服务器。
    ///
    /// # 返回值
    /// 如果关闭成功，则返回空结果，否则返回错误。
    pub fn server_kill(&mut self) -> Result<()> {
        let command = "host:kill";
        self.stream.send_cmd_then_check_okay(command)?;
        Ok(())
    }

    /// 连接到指定的 ADB 设备。
    ///
    /// # 参数
    /// - `serial`: 设备的序列号，用于指定要连接的设备。
    ///
    /// # 返回值
    /// 返回连接结果的字符串表示，如果连接失败，则返回错误。
    pub fn connect_device(&mut self, serial: &str) -> Result<String> {
        let command = format!("host:connect:{}", serial);
        self.stream.send_cmd_then_check_okay(&command)?;
        let result = self.stream.read_string_block()?;
        Ok(result)
    }

    /// 断开与指定 ADB 设备的连接。
    ///
    /// # 参数
    /// - `serial`: 设备的序列号，用于指定要断开连接的设备。
    ///
    /// # 返回值
    /// 返回断开连接结果的字符串表示，如果断开连接失败，则返回错误。
    pub fn disconnect_device(&mut self, serial: &str) -> Result<String> {
        if serial.is_empty() {
            return Err(anyhow!("serial is empty"));
        }
        let command = format!("host:disconnect:{}", serial);
        self.stream.send_cmd_then_check_okay(&command)?;
        Ok(self.stream.read_string_block()?)
    }
}
