use crate::utils::start_adb_server;
use log::error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;
use anyhow::Context;

const DEFAULT_ADB_PORT: u16 = 5037;
const DEFAULT_ADB_TIMEOUT: u64 = 3;

const DEFAULT_ADB_HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

///
/// Adb Socket相关配置
/// addr: adb server 地址, 默认值 127.0.0.1:5037
/// timeout: socket timeout, 默认值 3
#[derive(Clone, Debug)]
pub struct AdbSocketConfig {
    pub addr: SocketAddr,
    pub timeout: u64,
}
///
/// AdbSocketConfig 默认配置
impl Default for AdbSocketConfig {
    fn default() -> Self {
        AdbSocketConfig {
            addr: SocketAddr::new(
                IpAddr::V4(DEFAULT_ADB_HOST),
                DEFAULT_ADB_PORT,
            ),
            timeout: DEFAULT_ADB_TIMEOUT,
        }
    }
}

impl AdbSocketConfig {

    /**
     * 创建一个新的ADB客户端实例。
     *
     * @param socket_addr ADB服务器的Socket地址，可以是任何形式可以转换为SocketAddr的类型。
     * @param timeout 连接超时时间，以秒为单位。如果未指定，则使用默认超时时间。
     * @return 返回配置好的AdbSocketConfig
     */
    pub fn new<T: Into<SocketAddr>>(socket_addr: T, timeout: Option<u64>) -> Self {
        let timeout = timeout.map_or(DEFAULT_ADB_TIMEOUT, |t| t);
        Self {
            addr: socket_addr.into(),
            timeout,
        }
    }

    ///
    /// 设置超时时间
    pub fn set_timeout(&mut self, timeout: u64) {
        self.timeout = timeout;
    }


    ///
    /// 使用配置连接到Adb Server
    /// 本方法不可信, 请使用safe_connect
    pub fn create_socket(&self) -> anyhow::Result<TcpStream> {
        let stream = TcpStream::connect(self.addr)?;

        Ok(stream)
    }

    /// 安全尝试连接到ADB服务器。
    ///
    /// # 参数
    /// null
    ///
    /// # 返回值
    /// 返回一个`TcpStream`的`anyhow::Result`。成功时，`Ok`包含连接的TCP流；失败时，`Err`包含错误信息。
    pub fn safe_connect(&self) -> anyhow::Result<TcpStream> {
        let stream = match self.create_socket() {
            Ok(stream) => stream,
            Err(e) => {
                error!(
                    "Connect To Adb Failed, Try To Start Adb Server >>> {:#?}",
                    e
                );
                start_adb_server();
                self.create_socket().context("Failed to create TCP stream after starting ADB server")?
            }
        };
        stream.set_read_timeout(Some(Duration::new(self.timeout, 0)))?;
        return Ok(stream);
    }
}
