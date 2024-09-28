use crate::client::adb_device::AdbDevice;
use crate::client::AdbProtocol;
use anyhow::{anyhow, Context};

#[cfg(feature = "tokio")]
use futures_core::Stream;
#[cfg(feature = "tokio")]
use futures_util::stream;
use log::info;
#[cfg(feature = "tokio")]
use tokio::net::{TcpStream, ToSocketAddrs};

#[cfg(feature = "blocking")]
use std::net::{TcpStream, ToSocketAddrs};

#[cfg(feature = "tokio")]
pub struct AdbClient {
    stream: TcpStream,
}

#[cfg(feature = "tokio")]
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
            .context("Get Device List Error")
            .unwrap();
        stream::iter(devices)
    }

    /// 获取 ADB 服务器的版本号。
    ///
    /// # 返回值
    /// 返回服务器的版本号字符串，如果获取失败，则返回错误。
    pub async fn server_version(&mut self) -> anyhow::Result<String> {
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
    pub async fn server_kill(&mut self) -> anyhow::Result<()> {
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
    pub async fn connect_device(&mut self, serial: &str) -> anyhow::Result<String> {
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
    pub async fn disconnect_device(&mut self, serial: &str) -> anyhow::Result<String> {
        if serial.is_empty() {
            return Err(anyhow!("serial is empty"));
        }
        let command = format!("host:disconnect:{}", serial);
        self.stream.send_cmd_then_check_okay(&command).await?;
        Ok(self.stream.read_string_block().await?)
    }

    pub async fn list_devices(
        &mut self,
    ) -> anyhow::Result<Vec<AdbDevice<impl ToSocketAddrs + Clone>>> {
        self.stream.send_cmd_then_check_okay("host:devices").await?;
        let resp = self.stream.read_string_block().await?;
        let mut devices = vec![];
        if !resp.is_empty() {
            resp.lines().into_iter().for_each(|line| {
                let parts: Vec<&str> = line.split("\t").collect();
                if !parts.is_empty() {
                    // devices.push(self.device(parts[0]));
                    info!(">>>>>>> Device: {:#?}", parts)
                }
            })
        };
        Ok(devices)
    }
}

#[cfg(feature = "blocking")]
pub struct AdbClient {
    stream: TcpStream,
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
    ) -> anyhow::Result<impl Iterator<Item = AdbDevice<impl ToSocketAddrs + Clone>>> {
        Ok(self
            .list_devices()
            .context("Get Device List Error")?
            .into_iter())
    }

    pub fn list_devices(&mut self) -> anyhow::Result<Vec<AdbDevice<impl ToSocketAddrs + Clone>>> {
        self.stream.send_cmd_then_check_okay("host:devices")?;
        let resp = self.stream.read_string_block()?;
        let mut devices = vec![];
        if !resp.is_empty() {
            resp.lines().into_iter().for_each(|line| {
                let parts: Vec<&str> = line.split("\t").collect();
                if !parts.is_empty() {
                    let device =
                        AdbDevice::new(parts[0], self.stream.local_addr().unwrap().clone());
                    devices.push(device)
                    // devices.push(self.device(parts[0]));
                }
            })
        };
        Ok(devices)
    }

    /// 获取 ADB 服务器的版本号。
    ///
    /// # 返回值
    /// 返回服务器的版本号字符串，如果获取失败，则返回错误。
    pub fn server_version(&mut self) -> anyhow::Result<String> {
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
    pub fn server_kill(&mut self) -> anyhow::Result<()> {
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
    pub fn connect_device(&mut self, serial: &str) -> anyhow::Result<String> {
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
    pub fn disconnect_device(&mut self, serial: &str) -> anyhow::Result<String> {
        if serial.is_empty() {
            return Err(anyhow!("serial is empty"));
        }
        let command = format!("host:disconnect:{}", serial);
        self.stream.send_cmd_then_check_okay(&command)?;
        Ok(self.stream.read_string_block()?)
    }
}
