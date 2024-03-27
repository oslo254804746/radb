use crate::client::adb_device::AdbDevice;
use crate::connections::adb_protocol::AdbProtocolStreamHandler;
use crate::connections::adb_socket_config::AdbSocketConfig;
use anyhow::{anyhow, Context};
use std::net::{IpAddr, SocketAddr, TcpStream};

/// AdbConnection 结构体定义了与 ADB 服务器的连接。
pub struct AdbConnection {
    pub stream: TcpStream, // TCP 流，用于与 ADB 服务器进行通信。
    pub config: AdbSocketConfig, // ADB 插口配置，包含连接的地址和超时设置。
}

/// 实现 AdbProtocolStreamHandler 协议，提供对 TCP 流的基本操作。
impl AdbProtocolStreamHandler for AdbConnection {
    fn stream(&mut self) -> &mut TcpStream {
        &mut self.stream
    }
}

impl AdbConnection {

    /// 创建一个新的 AdbConnection 实例。
    ///
    /// # 参数
    /// - `sock_addr`: 可选的 Socket 地址，用于指定连接的 ADB 服务器地址。
    /// - `timeout`: 可选的超时时间（以秒为单位），用于设置连接的超时。
    ///
    /// # 返回值
    /// 返回一个建立好的 AdbConnection 实例，如果无法建立连接，则返回错误。
    pub fn new(sock_addr: Option<SocketAddr>, timeout: Option<u64>) -> anyhow::Result<Self>{
        if sock_addr.is_none() & timeout.is_none(){
            return Self::default()
        }else{
            let config = match sock_addr {
                Some(sock_addr) => AdbSocketConfig::new(sock_addr, timeout),
                _ => AdbSocketConfig::new(SocketAddr::new("127.0.0.1:5037".parse::<IpAddr>().unwrap(), 5037), timeout)
            };
            let stream =  config.safe_connect()?;
            Ok(Self{stream, config})
        }
    }

    /// 创建一个默认配置的 AdbConnection 实例，连接到标准的 ADB 服务器地址。
    ///
    /// # 返回值
    /// 返回一个默认配置的 AdbConnection 实例，如果无法建立连接，则返回错误。
    pub fn default() -> anyhow::Result<Self> {
        let config = AdbSocketConfig::default();
        let stream = config.safe_connect()?;
        Ok(Self { config, stream })
    }

    /// 列出所有连接的 ADB 设备。
    ///
    /// # 返回值
    /// 返回一个包含所有设备的向量，如果获取设备列表失败，则返回错误。
    pub fn list_devices(&mut self) -> anyhow::Result<Vec<AdbDevice>> {
        self.send_command("host:devices")?;
        self.check_okay()?;
        let resp = self.read_string_block()?;
        let mut devices = vec![];
        if !resp.is_empty() {
            resp.lines().into_iter().for_each(|line| {
                let parts: Vec<&str> = line.split("\t").collect();
                if !parts.is_empty() {
                    devices.push(self.device(parts[0]));
                }
            })
        };
        Ok(devices)
    }

    pub fn send_cmd_then_check_okay(
        &mut self,
        command: &str,
    ) -> anyhow::Result<()> {
        self.send_command(command)?;
        self.check_okay()?;

        Ok(())
    }

    /// 以迭代器的形式列出所有连接的 ADB 设备。
    ///
    /// # 返回值
    /// 返回一个设备迭代器，如果获取设备列表失败，则返回错误。
    pub fn iter_devices(&mut self) -> anyhow::Result<impl Iterator<Item = AdbDevice>> {
        Ok(self
            .list_devices()
            .context("Get Device List Error")?
            .into_iter())
    }

    /// 获取 ADB 服务器的版本号。
    ///
    /// # 返回值
    /// 返回服务器的版本号字符串，如果获取失败，则返回错误。
    pub fn server_version(&mut self) -> anyhow::Result<String> {
        let command = "host:version";
        self.send_cmd_then_check_okay(command, )?;
        let version_string = self.read_string_block()?;
        let version = usize::from_str_radix(&version_string, 16)?;
        Ok(version.to_string())
    }

    /// 关闭 ADB 服务器。
    ///
    /// # 返回值
    /// 如果关闭成功，则返回空结果，否则返回错误。
    pub fn server_kill(&mut self) -> anyhow::Result<()> {
        let command = "host:kill";
        self.send_cmd_then_check_okay(command, )?;
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
        self.send_cmd_then_check_okay(&command, )?;
        let result = self.read_string_block()?;
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
        self.send_cmd_then_check_okay(&command, )?;
        Ok(self.read_string_block()?)
    }

    /// 根据设备序列号创建一个 AdbDevice 实例。
    ///
    /// # 参数
    /// - `serial`: 设备的序列号。
    ///
    /// # 返回值
    /// 返回一个新的 AdbDevice 实例。
    pub fn device(&mut self, serial: &str) -> AdbDevice {
        AdbDevice::new_device(&serial, self.config.clone())
    }

}


#[test]
fn test_adb() {
    let mut adb = AdbConnection::default().unwrap();
    let devices = adb.list_devices();
    println!("{:?}", devices);
}
