use crate::client::adb_device::AdbDevice;
use std::fmt::Debug;

#[cfg(feature = "tokio_async")]
use tokio::net::{TcpStream, ToSocketAddrs};

use crate::errors::AdbResult;

#[cfg(feature = "blocking")]
use std::net::{TcpStream, ToSocketAddrs};

const DEFAULT_ADB_ADDR: &'static str = "127.0.0.1:5037";

pub struct AdbClient {
    pub stream: TcpStream,
}

impl AdbClient {
    pub fn parse_device_list_lines<T>(
        lines: &str,
        addr: T,
    ) -> AdbResult<Vec<AdbDevice<impl ToSocketAddrs + Clone + Debug>>>
    where
        T: ToSocketAddrs + Clone + Debug,
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
pub mod async_impl {
    use crate::client::adb_client::DEFAULT_ADB_ADDR;
    use crate::client::{AdbClient, AdbDevice};
    use crate::errors::{AdbError, AdbResult};
    use crate::protocols::AdbProtocol;
    use anyhow::anyhow;
    use futures_core::Stream;
    use futures_util::stream;
    use std::fmt::Debug;
    use tokio::net::{TcpStream, ToSocketAddrs};

    impl AdbClient {
        pub async fn default() -> Self {
            let stream = TcpStream::connect(DEFAULT_ADB_ADDR).await.unwrap();
            Self { stream }
        }

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
        ) -> impl Stream<Item = AdbDevice<impl ToSocketAddrs + Clone + Debug>> {
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
        pub async fn server_version(&mut self) -> AdbResult<String> {
            let command = "host:version";
            self.stream.send_cmd_then_check_okay(command).await?;
            let version_string = self.stream.read_response().await?;
            let version = usize::from_str_radix(&version_string, 16)?;
            Ok(version.to_string())
        }

        /// 关闭 ADB 服务器。
        ///
        /// # 返回值
        /// 如果关闭成功，则返回空结果，否则返回错误。
        pub async fn server_kill(&mut self) -> AdbResult<()> {
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
        pub async fn connect_device(&mut self, serial: &str) -> AdbResult<String> {
            let command = format!("host:connect:{}", serial);
            self.stream.send_cmd_then_check_okay(&command).await?;
            let result = self.stream.read_response().await?;
            Ok(result)
        }

        /// 断开与指定 ADB 设备的连接。
        ///
        /// # 参数
        /// - `serial`: 设备的序列号，用于指定要断开连接的设备。
        ///
        /// # 返回值
        /// 返回断开连接结果的字符串表示，如果断开连接失败，则返回错误。
        pub async fn disconnect_device(&mut self, serial: &str) -> AdbResult<String> {
            if serial.is_empty() {
                return Err(AdbError::from_display("serial is empty"));
            }
            let command = format!("host:disconnect:{}", serial);
            self.stream.send_cmd_then_check_okay(&command).await?;
            Ok(self.stream.read_response().await?)
        }

        pub async fn list_devices(
            &mut self,
        ) -> AdbResult<Vec<AdbDevice<impl ToSocketAddrs + Clone + Debug>>> {
            self.stream.send_cmd_then_check_okay("host:devices").await?;
            let resp = self.stream.read_response().await?;
            Self::parse_device_list_lines(&resp, self.stream.peer_addr()?.clone())
        }
    }
}

#[cfg(feature = "blocking")]
pub mod blocking_impl {
    use crate::client::adb_client::DEFAULT_ADB_ADDR;
    use crate::client::{AdbClient, AdbDevice};
    use crate::errors::{AdbError, AdbResult};
    use crate::protocols::AdbProtocol;
    use std::fmt::Debug;
    use std::net::{TcpStream, ToSocketAddrs};

    impl Default for AdbClient {
        fn default() -> Self {
            Self::new(DEFAULT_ADB_ADDR)
        }
    }

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
        ) -> AdbResult<impl Iterator<Item = AdbDevice<impl ToSocketAddrs + Clone + Debug>>>
        {
            Ok(self.list_devices()?.into_iter())
        }

        pub fn list_devices(
            &mut self,
        ) -> AdbResult<Vec<AdbDevice<impl ToSocketAddrs + Clone + Debug>>> {
            self.stream.send_cmd_then_check_okay("host:devices")?;
            let resp = self.stream.read_response()?;
            Self::parse_device_list_lines(&resp, self.stream.peer_addr()?.clone())
        }

        /// 获取 ADB 服务器的版本号。
        ///
        /// # 返回值
        /// 返回服务器的版本号字符串，如果获取失败，则返回错误。
        pub fn server_version(&mut self) -> AdbResult<String> {
            let command = "host:version";
            self.stream.send_cmd_then_check_okay(command)?;
            let version_string = self.stream.read_response()?;
            let version = usize::from_str_radix(&version_string, 16)?;
            Ok(version.to_string())
        }

        /// 关闭 ADB 服务器。
        ///
        /// # 返回值
        /// 如果关闭成功，则返回空结果，否则返回错误。
        pub fn server_kill(&mut self) -> AdbResult<()> {
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
        pub fn connect_device(&mut self, serial: &str) -> AdbResult<String> {
            let command = format!("host:connect:{}", serial);
            self.stream.send_cmd_then_check_okay(&command)?;
            let result = self.stream.read_response()?;
            Ok(result)
        }

        /// 断开与指定 ADB 设备的连接。
        ///
        /// # 参数
        /// - `serial`: 设备的序列号，用于指定要断开连接的设备。
        ///
        /// # 返回值
        /// 返回断开连接结果的字符串表示，如果断开连接失败，则返回错误。
        pub fn disconnect_device(&mut self, serial: &str) -> AdbResult<String> {
            if serial.is_empty() {
                return Err(AdbError::from_display("serial is empty"));
            }
            let command = format!("host:disconnect:{}", serial);
            self.stream.send_cmd_then_check_okay(&command)?;
            Ok(self.stream.read_response()?)
        }
    }
}
