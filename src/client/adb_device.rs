use std::collections::HashMap;
use std::fmt::Debug;

use crate::beans::app_info::AppInfo;
use once_cell::sync::Lazy;

use crate::beans::ForwardItem;
use crate::errors::{AdbError, AdbResult};
use regex::Regex;
#[cfg(feature = "blocking")]
use std::net::ToSocketAddrs;
#[cfg(feature = "tokio_async")]
use tokio::net::ToSocketAddrs;

static IP_REGEXES: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    vec![
        (
            Regex::new(r"inet\s+addr:([\d.]+)").unwrap(),
            "ifconfig format",
        ),
        (
            Regex::new(r"inet\s+([\d.]+)/\d+").unwrap(),
            "ip command format",
        ),
        (
            Regex::new(r"inet\s+([\d.]+)\s+netmask").unwrap(),
            "alternative ifconfig format",
        ),
    ]
});
/// 从输出中提取IP地址的辅助函数
fn extract_ip_from_output(output: &str) -> Option<String> {
    for (regex, _description) in IP_REGEXES.iter() {
        if let Some(captures) = regex.captures(output) {
            if let Some(ip_match) = captures.get(1) {
                let ip = ip_match.as_str();
                // 验证IP地址格式
                if is_valid_ipv4(ip) {
                    return Some(ip.to_string());
                }
            }
        }
    }
    None
}

fn extract_forward_item_from_output(output: String) -> AdbResult<Vec<ForwardItem>> {
    let target = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                Some(ForwardItem::new(parts[0], parts[1], parts[2]))
            } else {
                log::warn!("Invalid forward list line: {}", line);
                None
            }
        })
        .collect();
    Ok(target)
}

/// 验证IPv4地址格式
fn is_valid_ipv4(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return false;
    }

    parts.iter().all(|&part| {
        if let Ok(num) = part.parse::<u8>() {
            num <= 255
        } else {
            false
        }
    })
}

/// 从TCP规格字符串中提取端口号
fn extract_port_from_tcp_spec(tcp_spec: &str) -> Option<u16> {
    if tcp_spec.starts_with("tcp:") {
        tcp_spec[4..].parse().ok()
    } else {
        None
    }
}

/// 转义shell参数
fn escape_shell_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }

    // 如果不包含特殊字符，直接返回
    if !arg.chars().any(|c| " \"'\\$`(){}[]|&;<>?*~".contains(c)) {
        return arg.to_string();
    }

    // 使用双引号包围并转义内部的特殊字符
    let mut escaped = String::with_capacity(arg.len() + 10);
    escaped.push('"');

    for c in arg.chars() {
        match c {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '$' => escaped.push_str("\\$"),
            '`' => escaped.push_str("\\`"),
            _ => escaped.push(c),
        }
    }

    escaped.push('"');
    escaped
}

/// 提取应用版本信息
fn extract_app_version_info(output: &str, app_info: &mut AppInfo) {
    // 版本名称
    if let Ok(version_name_regex) = Regex::new(r"versionName=(\S+)") {
        if let Some(cap) = version_name_regex.captures(output) {
            if let Some(version_name) = cap.get(1) {
                app_info.version_name = Some(version_name.as_str().to_string());
            }
        }
    }

    // 版本代码
    if let Ok(version_code_regex) = Regex::new(r"versionCode=(\d+)") {
        if let Some(cap) = version_code_regex.captures(output) {
            if let Some(version_code) = cap.get(1) {
                if let Ok(code) = version_code.as_str().parse::<u32>() {
                    app_info.version_code = Some(code);
                }
            }
        }
    }
}

/// 提取应用签名信息
fn extract_app_signature(output: &str, app_info: &mut AppInfo) {
    if let Ok(signature_regex) = Regex::new(r"PackageSignatures\{[^}]*\[([^]]+)]") {
        if let Some(cap) = signature_regex.captures(output) {
            if let Some(signature) = cap.get(1) {
                app_info.signature = Some(signature.as_str().to_string());
            }
        }
    }
}

/// 提取应用标志信息
fn extract_app_flags(output: &str, app_info: &mut AppInfo) {
    if let Ok(flags_regex) = Regex::new(r"pkgFlags=\[\s*([^]]+)\s*]") {
        if let Some(cap) = flags_regex.captures(output) {
            if let Some(flags_str) = cap.get(1) {
                let flags: Vec<String> = flags_str
                    .as_str()
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                app_info.flags = flags;
            }
        }
    }
}

/// 提取应用时间戳信息
fn extract_app_timestamps(output: &str, app_info: &mut AppInfo) {
    use chrono::DateTime;
    use std::str::FromStr;

    // 首次安装时间
    if let Ok(first_install_regex) = Regex::new(r"firstInstallTime=([\d-]+\s+[:\d]+)") {
        if let Some(cap) = first_install_regex.captures(output) {
            if let Some(time_str) = cap.get(1) {
                if let Ok(datetime) = DateTime::from_str(time_str.as_str()) {
                    app_info.first_install_time = Some(datetime);
                }
            }
        }
    }

    // 最后更新时间
    if let Ok(last_update_regex) = Regex::new(r"lastUpdateTime=([\d-]+\s+[:\d]+)") {
        if let Some(cap) = last_update_regex.captures(output) {
            if let Some(time_str) = cap.get(1) {
                if let Ok(datetime) = DateTime::from_str(time_str.as_str()) {
                    app_info.last_update_time = Some(datetime);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct AdbDevice<T>
where
    T: ToSocketAddrs + Clone + Debug,
{
    pub serial: Option<String>,   // 设备的序列号，唯一标识一个设备。
    pub transport_id: Option<u8>, // 设备的传输ID，用于识别设备在系统中的传输方式。
    pub properties: HashMap<String, String>, // 设备的属性，以键值对形式存储，可包含多种设备信息。
    pub addr: T,
}

impl<T> AdbDevice<T>
where
    T: ToSocketAddrs + Clone + Debug,
{
    pub fn new<U>(serial: U, addr: T) -> Self
    where
        U: Into<String>,
    {
        AdbDevice {
            serial: Some(serial.into()),
            transport_id: None,
            properties: HashMap::new(),
            addr,
        }
    }

    /// 获取打开设备的传输前缀。
    ///
    /// 根据提供的命令和设备的序列号或传输ID，构建并返回一个特定格式的字符串。
    /// 如果提供了命令，则格式为 `host-transport-id:传输ID:命令` 或 `host-serial:序列号:命令`。
    /// 如果没有提供命令，则格式为 `host-transport-id:传输ID` 或 `host:transport:序列号`。
    ///
    /// - `command`：可选的命令字符串，如果提供，将被添加到返回的字符串中。
    /// - 返回值：构建好的字符串，或者在某些条件下返回错误。
    pub fn get_open_transport_prefix(&self, command: Option<&str>) -> AdbResult<String> {
        // 检查序列号和传输ID，不能同时为None
        if self.serial.is_none() & self.transport_id.is_none() {
            return Err(AdbError::protocol_error(
                "TransportID and Serial Can Not Been None At Same Time",
            ));
        }
        // 根据是否提供了命令和是否有传输ID来决定返回字符串的格式
        if let Some(command) = command {
            if let Some(ref transport_id) = self.transport_id {
                Ok(format!("host-transport-id:{}:{}", transport_id, command))
            } else {
                Ok(format!(
                    "host-serial:{}:{}",
                    self.serial.clone().unwrap(),
                    command
                ))
            }
        } else {
            if let Some(ref transport_id) = self.transport_id {
                Ok(format!("host-transport-id:{}", transport_id))
            } else {
                Ok(format!("host:transport:{}", self.serial.clone().unwrap()))
            }
        }
    }

    pub fn list2cmdline(args: &[&str]) -> String {
        args.iter()
            .map(|&arg| escape_shell_arg(arg))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(feature = "tokio_async")]
pub mod async_impl {
    use crate::beans::command::AdbCommand;
    use crate::beans::{parse_file_info, AppInfo, FileInfo, ForwardItem, NetworkType};
    use crate::client::adb_device::{
        extract_app_flags, extract_app_signature, extract_app_timestamps, extract_app_version_info,
        extract_forward_item_from_output, extract_ip_from_output, extract_port_from_tcp_spec,
    };
    use crate::client::AdbDevice;
    use crate::errors::{AdbError, AdbResult};
    use crate::protocols::AdbProtocol;
    use crate::utils::adb_path;
    use anyhow::{anyhow, Context};
    use async_stream::stream;
    use futures_core::Stream;
    use futures_util::{pin_mut, StreamExt};
    use image::{io::Reader as ImageReader, RgbImage};
    use log::{error, info};
    use std::fmt::{Debug, Display};
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use std::{fs, time};
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader, BufStream};
    use tokio::net::{TcpStream, ToSocketAddrs};
    use tokio::process::Command;

    impl<T> AdbDevice<T>
    where
        T: ToSocketAddrs + Clone + Debug,
    {
        pub async fn open_transport(&mut self, command: Option<&str>) -> AdbResult<TcpStream> {
            // 获取打开传输的前缀，基于是否提供了命令和设备的序列号或传输ID。
            let prefix = self
                .get_open_transport_prefix(command)
                .map_err(|_| AdbError::parse_error("Get Open Transport Prefix Failed"))?;
            let mut stream = TcpStream::connect(self.addr.clone()).await?;
            stream.send_cmd_then_check_okay(&prefix).await?;
            Ok(stream)
        }

        async fn get_with_command(&mut self, command: &str) -> AdbResult<String> {
            let mut conn = self.open_transport(Some(command)).await?;
            let result = conn.read_response().await?;
            Ok(result)
        }

        ///
        /// 与 命令 adb get-state 相同  => device
        pub async fn get_state(&mut self) -> AdbResult<String> {
            self.get_with_command("get-state").await
        }

        ///
        /// adb get-serialno => emulator-5554
        pub async fn get_serialno(&mut self) -> AdbResult<String> {
            self.get_with_command("get-serialno").await
        }

        ///adb get-devpath
        pub async fn get_devpath(&mut self) -> AdbResult<String> {
            self.get_with_command("get-devpath").await
        }

        pub async fn get_features(&mut self) -> AdbResult<String> {
            self.get_with_command("get-features").await
        }

        /// 执行通过ADB shell命令流，并返回一个AdbConnection的实例。
        ///
        /// # 参数
        /// - `command`: 一个包含多个命令参数的字符串切片数组，每个元素都是一个命令参数。
        ///
        /// # 返回值
        /// - `AdbResult<AdbConnection>`: 如果命令成功执行，则返回一个AdbConnection的实例；
        ///                                  如果执行过程中出现错误，则返回错误信息。
        pub async fn shell_stream<T2: Into<AdbCommand>>(
            &mut self,
            command: T2,
        ) -> AdbResult<TcpStream> {
            // 打开与设备的传输通道
            let mut conn = self.open_transport(None).await?;
            let cmd = command.into();

            // 构造完整的ADB shell命令字符串
            let send_cmd = format!("shell:{}", cmd.get_command());

            // 发送命令并检查是否执行成功
            let _ = conn.send_cmd_then_check_okay(&send_cmd).await?;

            // 返回成功的AdbConnection实例
            Ok(conn)
        }

        /// 在设备或模拟器上执行Shell命令，并返回命令的输出。
        ///
        /// # 参数
        /// - `command`: 一个字符串切片数组，代表要执行的Shell命令及其参数。
        ///
        /// # 返回值
        /// - `AdbResult<String>`: 命令执行成功则返回命令的输出结果，如果执行过程中出现错误则返回错误信息。
        pub async fn shell<T2: Into<AdbCommand>>(&mut self, command: T2) -> AdbResult<String> {
            // 通过`shell_stream`方法执行命令，获取命令的输出流
            let mut s = self.shell_stream(command).await?;

            // 从输出流中读取直到流关闭的所有数据，并将其存储为字符串
            let output = s.read_until_close().await?;

            // 将读取到的命令输出返回
            Ok(output)
        }

        pub async fn shell_trim<T2: Into<AdbCommand>>(&mut self, command: T2) -> AdbResult<String> {
            let s = self.shell(command).await?;
            Ok(s.trim().to_string())
        }

        pub async fn forward(
            &mut self,
            local: &str,
            remote: &str,
            norebind: bool,
        ) -> AdbResult<()> {
            let mut args = vec!["forward"];
            if norebind {
                args.push("norebind");
            }
            let forward_str = format!("{};{}", local, remote);
            args.push(&forward_str);
            let full_cmd = args.join(":");
            if let Ok(_) = self.open_transport(Some(&full_cmd)).await {
                return Ok(());
            }
            Err(AdbError::from_display("Failed To Forward Port"))
        }

        pub async fn forward_list(&mut self) -> AdbResult<Vec<ForwardItem>> {
            let mut connection = self.open_transport(Some("list-forward")).await?;
            let content = connection.read_response().await?;
            extract_forward_item_from_output(content)
        }
        pub async fn forward_remote_port(&mut self, remote_port: u16) -> AdbResult<u16> {
            let remote = format!("tcp:{}", remote_port);

            // 检查是否已经存在转发
            if let Ok(existing_forwards) = self.forward_list().await {
                for item in existing_forwards {
                    if let Some(ref serial) = self.serial {
                        if item.serial == *serial && item.remote == remote {
                            if let Some(local_port) = extract_port_from_tcp_spec(&item.local) {
                                info!("Found existing forward: {} -> {}", item.local, item.remote);
                                return Ok(local_port);
                            }
                        }
                    }
                }
            }

            // 创建新的端口转发
            let local_port = crate::utils::get_free_port()?;
            let local = format!("tcp:{}", local_port);

            self.forward(&local, &remote, false)
                .await
                .context("Failed to create port forward")?;

            Ok(local_port)
        }
        pub async fn reverse(
            &mut self,
            remote: &str,
            local: &str,
            norebind: bool,
        ) -> AdbResult<()> {
            let mut args = vec!["forward"];
            if norebind {
                args.push("norebind");
            }
            args.push(local);
            args.push(";");
            args.push(remote);
            let full_cmd = args.join(":");
            self.open_transport(Some(&full_cmd)).await?;
            Ok(())
        }

        pub async fn create_connection<S: Display>(
            &mut self,
            network_type: NetworkType,
            address: S,
        ) -> AdbResult<TcpStream> {
            let mut connection = self.open_transport(None).await?;
            let cmd = match network_type {
                NetworkType::LocalAbstract | NetworkType::Unix => {
                    format!("{}{}", "localabstract:", address)
                }
                _ => format!("{}{}", network_type.to_string(), address),
            };
            connection
                .send_cmd_then_check_okay(&cmd)
                .await
                .map_err(|e| anyhow!("Send Command >> {:#?} and Check Okay Failed {} ", &cmd, e))?;
            Ok(connection)
        }
        pub async fn adb_output(&mut self, command: &[&str]) -> AdbResult<String> {
            let adb_ = adb_path()?;
            if adb_.exists() {
                if let Some(ref serial) = self.serial {
                    let cmd = Command::new(adb_)
                        .arg("-s")
                        .arg(serial)
                        .args(command)
                        .output()
                        .await?;
                    return Ok(String::from_utf8_lossy(&cmd.stdout).to_string());
                }
            };
            Err(AdbError::from_display("adb not found"))
        }

        pub async fn tcpip(&mut self, port: u16) -> AdbResult<String> {
            let mut connection = self.open_transport(None).await?;
            let cmd = format!("tcpip:{}", port);
            connection
                .send_cmd_then_check_okay(&cmd)
                .await
                .map_err(|e| anyhow!("Send Command >> {:#?} and Check Okay Failed {} ", &cmd, e))?;
            let resp = connection
                .read_until_close()
                .await
                .map_err(|e| anyhow!("Read Until Close Failed {}", e))?;
            Ok(resp)
        }

        pub async fn push(&mut self, local: &str, remote: &str) -> AdbResult<()> {
            if self.adb_output(&["push", local, remote]).await.is_ok() {
                info!("push {} to {} success", local, remote);
                return Ok(());
            }
            Err(AdbError::from_display("push error"))
        }
        pub async fn pull(&mut self, src: &str, dest: &PathBuf) -> AdbResult<usize> {
            let mut size = 0;
            let mut file = match File::open(dest) {
                Ok(mut file) => file,
                Err(_) => File::create(dest)?,
            };
            let _ = self.iter_content(src).await?.map(|x| {
                let data = x.unwrap();
                file.write_all(&data).unwrap();
                size += data.len();
            });
            Ok(size)
        }

        pub async fn iter_directory(
            &mut self,
            path: &str,
        ) -> AdbResult<impl Stream<Item = AdbResult<(Vec<u8>, String)>>> {
            let mut conn = self.prepare_sync(path, "LIST").await?;
            Ok(stream! {
                loop {
                    match conn.read_string(4).await{
                    Ok(data) => {
                        if data.eq("DONE") {
                            break
                        } else {
                            let mut current_data = conn.recv(16).await?;
                            let name_length_bytes = &current_data[12..=15];
                            let name_length = u32::from_le_bytes(name_length_bytes.try_into().unwrap());
                            let path = conn.read_string(name_length as usize).await?;
                            yield Ok((current_data, path))
                        }
                    },
                    Err(e) => {
                        yield Err(e);
                        break
                    }
                }

            }
            })
        }

        pub async fn exists(&mut self, path: &str) -> AdbResult<bool> {
            let file_info = self.stat(path).await?;
            if file_info.mtime != 0 {
                Ok(true)
            } else {
                Ok(false)
            }
        }

        pub async fn stat(&mut self, path: &str) -> AdbResult<FileInfo> {
            let mut conn = self.prepare_sync(path, "STAT").await?;
            let data = conn.read_string(4).await?;
            if data.eq("STAT") {
                let current_data = conn.recv(12).await?;
                return Ok(parse_file_info(current_data, path)?);
            };
            Err(AdbError::from_display("stat error"))
        }

        pub async fn list(&mut self, path: &str) -> AdbResult<Vec<FileInfo>> {
            let mut stream = self.iter_directory(path).await?;
            let mut files = vec![];
            pin_mut!(stream);
            while let Some(data) = stream.next().await {
                match data {
                    Ok((binary_data, path)) => {
                        if let Ok(file_info) = parse_file_info(binary_data, path) {
                            files.push(file_info);
                        }
                    }
                    Err(e) => {
                        error!("发生异常 {:#?}", e)
                    }
                }
            }
            Ok(files)
        }

        pub async fn read_text(
            &mut self,
            path: &str,
        ) -> AdbResult<impl Stream<Item = anyhow::Result<String>>> {
            let stream = self.iter_content(path).await?;
            Ok(stream! {
                pin_mut!(stream);
                while let Some(data)  = stream.next().await{
                    match data{
                    Ok(data) => {
                        yield Ok(String::from_utf8_lossy(&data).to_string())
                    },
                    Err(e) => {
                        yield Err(e);break;
                    }
                }

            }
            })
        }

        pub async fn prepare_sync(&mut self, path: &str, command: &str) -> AdbResult<TcpStream> {
            info!("Start Sync Path {:#?} With Command {:#?}", path, command);
            let mut conn = self.open_transport(None).await?;
            conn.send_cmd_then_check_okay("sync:").await?;
            let path_len = path.as_bytes().len() as u32;
            let mut total_byte = vec![];
            total_byte.extend_from_slice(command.as_bytes());
            total_byte.extend_from_slice(&path_len.to_le_bytes());
            total_byte.extend_from_slice(path.as_bytes());
            conn.send(&total_byte).await?;
            Ok(conn)
        }

        pub async fn iter_content(
            &mut self,
            path: &str,
        ) -> AdbResult<impl Stream<Item = anyhow::Result<Vec<u8>>>> {
            let mut connection = self.prepare_sync(path, "RECV").await?;
            Ok(stream! {
                            loop{
                                match connection.read_string(4).await {
                                    Err(e) => {
                                        yield Err(anyhow!("Read String Error {}", e));
                                        break;
                                    },
                                    Ok(data) =>  {
                                        let match_resp = match data.as_str() {
                                        "FAIL" => match connection.recv(4).await {
                                            Err(e) => {
                                                Err(anyhow!("Read String Error {}", e))
                                            },
                                            Ok(data) => {
                                                let str_size = u32::from_le_bytes(data.try_into().ok().unwrap()) as usize;
                                                let error_message = connection.read_string(str_size).await.ok().unwrap();
                                                error!("Sync Error With Error Message >>> {}", &error_message);
                                                Err(anyhow!("Read String Error {}", error_message))

                                            }
                                        },
                                        "DONE" => {
                                            Err(anyhow!("Read Done"))
                                        }
                                        "DATA" => match connection.recv(4).await {
                                            Ok(size) => {
                                                let str_size = u32::from_le_bytes(size.try_into().ok().unwrap()) as usize;
                                                let mut buffer = vec![0; str_size];
                                                match connection.read_exact(& mut buffer).await {
                                                    Ok(data) => Ok(buffer[..data].to_vec()),
                                                    Err(e) => Err(anyhow!("Read String Error {}", e)),
                                                }
                                            }
                                            Err(e) => Err(anyhow!("Read String Error {}", e)),
                                        },
                                        _ => Err(anyhow!("Read String Error ")),
                                    };
                                    if match_resp.is_err(){
                                        yield match_resp;
                                        break;
                                    }
                                yield match_resp
                            }
                                }
            }
                })
        }

        pub async fn screenshot(&mut self) -> AdbResult<RgbImage> {
            let src = "/sdcard/screen.png";
            self.shell(["screencap", "-p", src]).await?;
            let tmpdir = tempfile::tempdir().expect("Failed to create temporary directory");
            let target_path = tmpdir.path().join("tmp001.png");
            info!("Pull Image To {:#?}", &target_path);
            self.pull(src, &target_path).await?;
            self.shell(["rm", src]).await?;

            let image = ImageReader::open(&target_path)?
                .decode()
                .context("Fail to decode image")?;
            fs::remove_file(target_path).expect("Failed to remove file");
            Ok(image.into_rgb8())
        }

        pub async fn keyevent(&mut self, keycode: &str) -> AdbResult<String> {
            self.shell(["input", "keyevent", keycode]).await
        }

        pub async fn switch_screen(&mut self, status: bool) -> AdbResult<String> {
            if status == true {
                self.keyevent("224").await
            } else {
                self.keyevent("223").await
            }
        }

        pub async fn install(&mut self, path_or_url: &str) -> AdbResult<()> {
            let target_path =
                if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
                    let mut resp = reqwest::get(path_or_url)
                        .await
                        .context("Fail to get http response")?;
                    let response_bytes = resp.bytes().await.context("Fail to get bytes")?;
                    let temp_dir = tempfile::tempdir()?.path().join("tmp001.apk");
                    let mut fd = File::create(&temp_dir)?;
                    fd.write_all(&response_bytes)?;
                    let target_path = temp_dir.to_str().ok_or(anyhow!("fail to get path"))?;
                    info!(
                        "Save Http/s file to  <{:#?}> => dst: <{:#?}>",
                        &path_or_url, &target_path
                    );
                    target_path.to_string()
                } else {
                    path_or_url.to_string()
                };
            let dst = format!(
                "/data/local/tmp/tmp-{}.apk",
                (time::SystemTime::now()
                    .duration_since(time::UNIX_EPOCH)?
                    .as_millis())
            );
            info!("Pushing src: <{:#?}> => dst: <{:#?}> ", &path_or_url, &dst);
            self.push(&target_path, &dst).await?;
            match self.install_remote(&dst, true).await {
                Ok(resp) => {
                    info!("Install Apk Successed >> <{:#?}>", &resp);
                    return Ok(());
                }
                Err(e) => {
                    let error_string = format!("fail to install apk >>> {}", e);
                    error!("{}", &error_string);
                    Err(e)
                }
            }
        }
        pub async fn install_remote(&mut self, path: &str, clean: bool) -> AdbResult<String> {
            let args = ["pm", "install", "-r", "-t", path];
            let output = self.shell(args).await?;
            if !output.contains("Success") {
                return Err(anyhow!("fail to install").into());
            };
            if clean {
                self.shell(["rm", path]).await?;
            }
            Ok(output)
        }

        pub async fn switch_airplane_mode(&mut self, status: bool) -> AdbResult<String> {
            let mut base_setting_cmd = vec!["settings", "put", "global", "airplane_mode_on"];
            let mut base_am_cmd = vec![
                "am",
                "broadcast",
                "-a",
                "android.intent.action.AIRPLANE_MODE",
                "--ez",
                "state",
            ];
            if status == true {
                base_setting_cmd.push("1");
                base_am_cmd.push("true");
            } else {
                base_setting_cmd.push("0");
                base_am_cmd.push("false");
            }
            self.shell(base_setting_cmd).await?;
            self.shell(base_am_cmd).await
        }

        pub async fn switch_wifi(&mut self, status: bool) -> AdbResult<String> {
            let mut args = vec!["svc", "wifi"];
            if status == true {
                args.push("enable");
            } else {
                args.push("disable");
            };
            self.shell(args).await
        }

        pub async fn click(&mut self, x: i32, y: i32) -> AdbResult<String> {
            self.shell(["input", "tap", &x.to_string(), &y.to_string()])
                .await
        }

        pub async fn swipe(
            &mut self,
            x1: i32,
            y1: i32,
            x2: i32,
            y2: i32,
            duration: i32,
        ) -> AdbResult<String> {
            self.shell([
                "input",
                "swipe",
                &x1.to_string(),
                &y1.to_string(),
                &x2.to_string(),
                &y2.to_string(),
                &duration.to_string(),
            ])
            .await
        }

        pub async fn send_keys(&mut self, keys: &str) -> AdbResult<String> {
            self.shell(["input", "text", keys]).await
        }

        pub async fn wlan_ip(&mut self) -> AdbResult<String> {
            // 定义要尝试的网络接口和命令
            let interface_commands = [
                ("wlan0", vec!["ip", "addr", "show", "dev", "wlan0"]),
                ("wlan0", vec!["ifconfig", "wlan0"]),
                ("eth0", vec!["ip", "addr", "show", "dev", "eth0"]),
                ("eth0", vec!["ifconfig", "eth0"]),
                ("", vec!["ip", "route", "get", "1.1.1.1"]), // 获取默认路由的IP
            ];

            for (interface, cmd) in &interface_commands {
                if let Ok(result) = self.shell(cmd).await {
                    if let Some(ip) = extract_ip_from_output(&result) {
                        info!("Found IP {} on interface {}", ip, interface);
                        return Ok(ip);
                    }
                }
            }

            Err(AdbError::Unknown {
                message: "Failed to retrieve WLAN IP from any interface".to_string(),
            })
        }

        pub async fn uninstall(&mut self, package_name: &str) -> AdbResult<String> {
            self.shell(["am", "uninstall", package_name]).await
        }

        pub async fn app_start(&mut self, package_name: &str) -> AdbResult<String> {
            self.shell(["am", "start", "-n", package_name]).await
        }

        pub async fn app_stop(&mut self, package_name: &str) -> AdbResult<String> {
            self.shell(["am", "force-stop", package_name]).await
        }

        pub async fn app_clear_data(&mut self, package_name: &str) -> AdbResult<String> {
            self.shell(["pm", "clear", package_name]).await
        }

        pub async fn app_info(&mut self, package_name: &str) -> Option<AppInfo> {
            // 首先检查应用是否存在
            let output = self
                .shell(&["pm", "list", "packages", package_name])
                .await
                .ok()?;
            if !output.contains(&format!("package:{}", package_name)) {
                return None;
            }

            // 修复：dumpsys 命令拼写错误
            let app_info_output = self
                .shell(&["dumpsys", "package", package_name]) // 修复：pacakge -> package
                .await
                .ok()?;

            let mut app_info = AppInfo::new(package_name);

            // 使用更健壮的正则表达式匹配
            extract_app_version_info(&app_info_output, &mut app_info);
            extract_app_signature(&app_info_output, &mut app_info);
            extract_app_flags(&app_info_output, &mut app_info);
            extract_app_timestamps(&app_info_output, &mut app_info);

            Some(app_info)
        }
        pub async fn if_screen_on(&mut self) -> AdbResult<bool> {
            let resp = self.shell(["dumpsys", "power"]).await?;
            Ok(resp.contains("mHoldingDisplaySuspendBlocker=true"))
        }

        pub async fn remove(&mut self, path: &str) -> AdbResult<String> {
            self.shell_trim(["rm", path]).await
        }

        pub async fn get_sdk_version(&mut self) -> AdbResult<String> {
            self.shell_trim(["getprop", "ro.build.version.sdk"]).await
        }

        pub async fn get_android_version(&mut self) -> AdbResult<String> {
            self.shell_trim(["getprop", "ro.build.version.release"])
                .await
        }

        pub async fn get_device_model(&mut self) -> AdbResult<String> {
            self.shell_trim(["getprop", "ro.product.model"]).await
        }

        pub async fn get_device_brand(&mut self) -> AdbResult<String> {
            self.shell_trim(["getprop", "ro.product.brand"]).await
        }
        pub async fn get_device_manufacturer(&mut self) -> AdbResult<String> {
            self.shell_trim(["getprop", "ro.product.manufacturer"])
                .await
        }
        pub async fn get_device_product(&mut self) -> AdbResult<String> {
            self.shell_trim(["getprop", "ro.product.product"]).await
        }

        pub async fn get_device_abi(&mut self) -> AdbResult<String> {
            self.shell_trim(["getprop", "ro.product.cpu.abi"]).await
        }

        pub async fn get_device_gpu(&mut self) -> AdbResult<String> {
            let resp = self.shell(["dumpsys", "SurfaceFlinger"]).await;
            match resp {
                Ok(data) => {
                    for x in data.split("\n") {
                        if x.starts_with("GLES:") {
                            return Ok(x.to_string());
                        }
                    }
                }
                _ => {}
            }
            Err(AdbError::from_display("fail to get gpu"))
        }

        pub async fn logcat(
            &mut self,
            flush_exist: bool,
            extra_command: Option<AdbCommand>,
        ) -> AdbResult<impl Stream<Item = AdbResult<String>>> {
            if flush_exist {
                self.shell(["logcat", "-c"]).await?;
            }

            let cmd = if let Some(extra_cmd) = extra_command {
                extra_cmd
            } else {
                vec!["logcat", "-v", "time"].into()
            };

            let conn = self.shell_stream(cmd).await?;

            Ok(stream! {
                let mut reader = BufReader::new(conn);
                let mut buffer = String::new();
                loop {
                    buffer.clear();
                    match reader.read_line(&mut buffer).await {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            yield Ok(buffer.clone());
                        }
                        Err(e) => {
                            yield Err(AdbError::from(e));
                            break;
                        }
                    }
                }
            })
        }
    }
}

#[cfg(feature = "blocking")]
pub mod blocking_impl {
    use crate::beans::{parse_file_info, AppInfo, FileInfo, ForwardItem};
    use crate::client::adb_device::{
        extract_app_flags, extract_app_signature, extract_app_timestamps, extract_app_version_info,
        extract_forward_item_from_output, extract_ip_from_output, extract_port_from_tcp_spec,
    };
    use crate::client::AdbDevice;
    use crate::errors::{AdbError, AdbResult};
    use crate::protocols::AdbProtocol;
    use crate::utils::{adb_path, get_free_port};
    use anyhow::Context;

    use image::{io::Reader as ImageReader, RgbImage};
    use log::{error, info};
    use std::fmt::Debug;
    use std::fs::File;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::path::PathBuf;

    use crate::beans::command::AdbCommand;
    use std::sync::{Arc, RwLock};
    use std::{fs, time};

    pub struct LogcatIterator {
        reader: BufReader<TcpStream>,
        buffer: String,
    }

    impl Iterator for LogcatIterator {
        type Item = Result<String, std::io::Error>;

        fn next(&mut self) -> Option<Self::Item> {
            self.buffer.clear();
            match self.reader.read_line(&mut self.buffer) {
                Ok(0) => None, // EOF
                Ok(_) => Some(Ok(self.buffer.clone())),
                Err(e) => Some(Err(e)),
            }
        }
    }

    impl<T> AdbDevice<T>
    where
        T: ToSocketAddrs + Clone + Debug,
    {
        /// 打开一个Adb连接，通过给定的命令选项配置传输前缀。
        ///
        /// - `command`：可选的命令字符串，用于配置传输前缀。
        /// - 返回值：成功时返回一个`AdbConnection`实例，表示与设备的连接。
        pub fn open_transport(&mut self, command: Option<&str>) -> AdbResult<TcpStream> {
            // 获取打开传输的前缀，基于是否提供了命令和设备的序列号或传输ID。
            let prefix = self
                .get_open_transport_prefix(command)
                .context("Get Open Transport Prefix Failed")?;
            // 获取一个Adb连接。
            let mut stream = TcpStream::connect(&self.addr)?;
            stream.send_cmd_then_check_okay(&prefix).context(format!(
                "Send Command >> {:#?} and Check Okay Failed",
                &prefix
            ))?;
            Ok(stream)
        }

        fn get_with_command(&mut self, command: &str) -> AdbResult<String> {
            let mut conn = self.open_transport(Some(command))?;
            let result = conn.read_response()?;
            Ok(result)
        }

        pub fn get_state(&mut self) -> AdbResult<String> {
            self.get_with_command("get-state")
        }

        pub fn get_serialno(&mut self) -> AdbResult<String> {
            self.get_with_command("get-serialno")
        }

        pub fn get_devpath(&mut self) -> AdbResult<String> {
            self.get_with_command("get-devpath")
        }

        pub fn get_features(&mut self) -> AdbResult<String> {
            self.get_with_command("get-features")
        }

        /// 执行通过ADB shell命令流，并返回一个AdbConnection的实例。
        ///
        /// # 参数
        /// - `command`: 一个包含多个命令参数的字符串切片数组，每个元素都是一个命令参数。
        ///
        /// # 返回值
        /// - `AdbResult<AdbConnection>`: 如果命令成功执行，则返回一个AdbConnection的实例；
        ///                                  如果执行过程中出现错误，则返回错误信息。
        pub fn shell_stream<T2: Into<AdbCommand>>(&mut self, command: T2) -> AdbResult<TcpStream> {
            // 打开与设备的传输通道
            let mut conn = self.open_transport(None)?;
            let cmd = command.into().get_command();

            // 构造完整的ADB shell命令字符串
            let send_cmd = format!("shell:{}", cmd);

            // 发送命令并检查是否执行成功
            conn.send_cmd_then_check_okay(&send_cmd).context(format!(
                "Send Command >> {:#?} and Check Okay Failed",
                &send_cmd
            ))?;

            // 返回成功的AdbConnection实例
            Ok(conn)
        }

        /// 在设备或模拟器上执行Shell命令，并返回命令的输出。
        ///
        /// # 参数
        /// - `command`: 一个字符串切片数组，代表要执行的Shell命令及其参数。
        ///
        /// # 返回值
        /// - `AdbResult<String>`: 命令执行成功则返回命令的输出结果，如果执行过程中出现错误则返回错误信息。
        pub fn shell<T2: Into<AdbCommand>>(&mut self, command: T2) -> AdbResult<String> {
            // 通过`shell_stream`方法执行命令，获取命令的输出流
            let mut s = self.shell_stream(command)?;

            // 从输出流中读取直到流关闭的所有数据，并将其存储为字符串
            let output = s.read_until_close()?;

            // 将读取到的命令输出返回
            Ok(output)
        }
        pub fn shell_trim<T2: Into<AdbCommand>>(&mut self, command: T2) -> AdbResult<String> {
            let mut s = self.shell_stream(command)?;
            let output = s.read_until_close()?;
            Ok(output.trim().to_string())
        }

        pub fn forward(&mut self, local: &str, remote: &str, norebind: bool) -> AdbResult<()> {
            let mut args = vec!["forward"];
            if norebind {
                args.push("norebind");
            }
            let forward_str = format!("{};{}", local, remote);
            args.push(&forward_str);
            let full_cmd = args.join(":");
            if let Ok(_) = self.open_transport(Some(&full_cmd)) {
                return Ok(());
            }
            Err(AdbError::from_display("Failed To Forward Port"))
        }

        pub fn forward_list(&mut self) -> AdbResult<Vec<ForwardItem>> {
            let mut connection = self.open_transport(Some("list-forward"))?;
            let content = connection.read_response()?;
            extract_forward_item_from_output(content)
        }
        pub fn forward_remote_port(&mut self, remote: u16) -> AdbResult<u16> {
            let remote = format!("tcp:{}", remote);

            // 检查是否已经存在转发
            if let Ok(existing_forwards) = self.forward_list() {
                for item in existing_forwards {
                    if let Some(ref serial) = self.serial {
                        if item.serial == *serial && item.remote == remote {
                            if let Some(local_port) = extract_port_from_tcp_spec(&item.local) {
                                info!("Found existing forward: {} -> {}", item.local, item.remote);
                                return Ok(local_port);
                            }
                        }
                    }
                }
            }
            // 创建新的端口转发
            let local_port = get_free_port()?;
            let local = format!("tcp:{}", local_port);

            self.forward(&local, &remote, false)
                .context("Failed to create port forward")?;

            Ok(local_port)
        }

        pub fn reverse(&mut self, remote: &str, local: &str, norebind: bool) -> AdbResult<()> {
            let mut args = vec!["forward"];
            if norebind {
                args.push("norebind");
            }
            args.push(local);
            args.push(";");
            args.push(remote);
            let full_cmd = args.join(":");
            self.open_transport(Some(&full_cmd))?;
            Ok(())
        }

        pub fn adb_output(&mut self, command: &[&str]) -> AdbResult<String> {
            let adb_ = adb_path()?;
            if adb_.exists() {
                let mut cmd = std::process::Command::new(adb_.to_str().unwrap());
                cmd.arg("-s");
                cmd.arg(self.serial.as_ref().unwrap());
                for x in command {
                    cmd.arg(x);
                }
                info!("{:?}", &cmd);
                let output = cmd.output().expect("failed to execute process");
                return Ok(String::from_utf8_lossy(&output.stdout).to_string());
            };
            Err(AdbError::from_display("adb not found"))
        }

        pub fn tcpip(&mut self, port: u16) -> AdbResult<String> {
            let mut connection = self.open_transport(None)?;
            let cmd = format!("tcpip:{}", port);
            connection
                .send_cmd_then_check_okay(&cmd)
                .context(format!("Send Command >> {:#?} and Check Okay Failed", &cmd))?;
            let resp = connection
                .read_until_close()
                .context("Read Until Close Failed")?;
            Ok(resp)
        }
        pub fn push(&mut self, local: &str, remote: &str) -> AdbResult<()> {
            if self.adb_output(&["push", local, remote]).is_ok() {
                info!("push {} to {} success", local, remote);
                return Ok(());
            }
            Err(AdbError::from_display("push error"))
        }
        pub fn pull(&mut self, src: &str, dest: &PathBuf) -> AdbResult<usize> {
            let mut size = 0;
            let mut file = match File::open(dest) {
                Ok(mut file) => file,
                Err(_) => File::create(dest)?,
            };
            self.iter_content(src)?.for_each(|content| match content {
                Ok(content) => {
                    file.write_all(content.as_bytes()).unwrap();
                    size += content.len();
                }
                Err(_) => {}
            });
            Ok(size)
        }

        pub fn iter_directory(&mut self, path: &str) -> AdbResult<impl Iterator<Item = FileInfo>> {
            let mut conn = self.prepare_sync(path, "LIST")?;
            Ok(std::iter::from_fn(move || {
                let data = conn.read_string(4).ok()?;
                return if data.eq("DONE") {
                    None
                } else {
                    let mut current_data = conn.recv(16).ok()?;
                    let name_length_bytes = &current_data[12..=15];
                    let name_length = u32::from_le_bytes(name_length_bytes.try_into().unwrap());
                    let path = conn.read_string(name_length as usize).ok()?;
                    Some(parse_file_info(current_data, path).ok()?)
                };
            }))
        }

        pub fn exists(&mut self, path: &str) -> AdbResult<bool> {
            let file_info = self.stat(path)?;
            if file_info.mtime != 0 {
                Ok(true)
            } else {
                Ok(false)
            }
        }

        pub fn stat(&mut self, path: &str) -> AdbResult<FileInfo> {
            let mut conn = self.prepare_sync(path, "STAT")?;
            let data = conn.read_string(4)?;
            if data.eq("STAT") {
                let current_data = conn.recv(12)?;
                return Ok(parse_file_info(current_data, path)?);
            };
            Err(AdbError::from_display("stat error"))
        }

        pub fn list(&mut self, path: &str) -> AdbResult<Vec<FileInfo>> {
            Ok(self
                .iter_directory(path)
                .context("Iter Directory Error")?
                .collect::<Vec<FileInfo>>())
        }

        pub fn read_text(&mut self, path: &str) -> AdbResult<String> {
            let data = self
                .iter_content(path)?
                .map(|x| x.unwrap_or_else(|_| "".to_string()))
                .collect::<Vec<String>>();
            Ok(data.join(""))
        }

        pub fn prepare_sync(&mut self, path: &str, command: &str) -> AdbResult<TcpStream> {
            info!("Start Sync Path {:#?} With Command {:#?}", path, command);
            let mut conn = self.open_transport(None)?;
            conn.send_cmd_then_check_okay("sync:")
                .context("Start Sync Error")?;
            let path_len = path.as_bytes().len() as u32;
            let mut total_byte = vec![];
            total_byte.extend_from_slice(command.as_bytes());
            total_byte.extend_from_slice(&path_len.to_le_bytes());
            total_byte.extend_from_slice(path.as_bytes());
            conn.send(&total_byte)?;
            Ok(conn)
        }

        pub fn iter_content(
            &mut self,
            path: &str,
        ) -> AdbResult<impl Iterator<Item = AdbResult<String>>> {
            if let Ok(mut connection) = self.prepare_sync(path, "RECV") {
                let mut done = false;
                return Ok(std::iter::from_fn(move || {
                    if done {
                        return None;
                    }
                    return match connection.read_string(4) {
                        Err(_) => None,
                        Ok(data) => match data.as_str() {
                            "FAIL" => match connection.recv(4) {
                                Err(_) => None,
                                Ok(data) => {
                                    let str_size =
                                        u32::from_le_bytes(data.try_into().ok()?) as usize;
                                    let error_message = connection.read_string(str_size).ok()?;
                                    error!(
                                        "Sync Error With Error Message >>> {:#?}",
                                        error_message
                                    );
                                    None
                                }
                            },
                            "DONE" => {
                                done = true;
                                None
                            }
                            "DATA" => match connection.recv(4) {
                                Ok(size) => {
                                    let str_size =
                                        u32::from_le_bytes(size.try_into().ok()?) as usize;
                                    match connection.read_string(str_size) {
                                        Ok(data) => Some(Ok(data)),
                                        Err(_) => None,
                                    }
                                }
                                Err(_) => None,
                            },
                            _ => None,
                        },
                    };
                }));
            }
            Err(AdbError::from_display("iter_content error"))
        }

        pub fn screenshot(&mut self) -> AdbResult<RgbImage> {
            let src = "/sdcard/screen.png";
            self.shell(["screencap", "-p", src])?;
            let tmpdir = tempfile::tempdir().expect("Failed to create temporary directory");
            let target_path = tmpdir.path().join("tmp001.png");
            info!("Pull Image To {:#?}", &target_path);
            self.pull(src, &target_path)?;
            self.shell(["rm", src])?;

            let image = ImageReader::open(&target_path)?
                .decode()
                .context("Decode Image Error")?;
            fs::remove_file(target_path).expect("Failed to remove file");
            Ok(image.into_rgb8())
        }

        pub fn keyevent(&mut self, keycode: &str) -> AdbResult<String> {
            self.shell(["input", "keyevent", keycode])
        }

        pub fn switch_screen(&mut self, status: bool) -> AdbResult<String> {
            if status == true {
                self.keyevent("224")
            } else {
                self.keyevent("223")
            }
        }

        pub fn install(&mut self, path_or_url: &str) -> AdbResult<()> {
            let target_path = if path_or_url.starts_with("http://")
                || path_or_url.starts_with("https://")
            {
                let mut resp = reqwest::blocking::get(path_or_url).context("Http Request Error")?;
                let mut buffer = Vec::new();
                resp.read_to_end(&mut buffer)?;
                let temp_dir = tempfile::tempdir()?.path().join("tmp001.apk");
                let mut fd = File::create(&temp_dir)?;
                fd.write_all(&buffer)?;
                let target_path = temp_dir.to_str().ok_or(AdbError::file_operation_failed(
                    "getTempDir",
                    "fail to get path",
                ))?;
                info!(
                    "Save Http/s file to  <{:#?}> => dst: <{:#?}>",
                    &path_or_url, &target_path
                );
                target_path.to_string()
            } else {
                path_or_url.to_string()
            };
            let dst = format!(
                "/data/local/tmp/tmp-{}.apk",
                (time::SystemTime::now()
                    .duration_since(time::UNIX_EPOCH)?
                    .as_millis())
            );
            info!("Pushing src: <{:#?}> => dst: <{:#?}> ", &path_or_url, &dst);
            self.push(&target_path, &dst)?;
            let install_resp = self.install_remote(&dst, true);
            info!("Install Apk Result {:#?}", &install_resp);
            if let Ok(resp) = install_resp {
                info!("Install Apk Successed >> <{:#?}>", &resp);
                return Ok(());
            }
            Err(AdbError::from_display("fail to install apk"))
        }
        pub fn install_remote(&mut self, path: &str, clean: bool) -> AdbResult<String> {
            let args = ["pm", "install", "-r", "-t", path];
            let output = self.shell(args)?;
            if !output.contains("Success") {
                return Err(AdbError::from_display("fail to install"));
            };
            if clean {
                self.shell(["rm", path])?;
            }
            Ok(output)
        }

        pub fn switch_airplane_mode(&mut self, status: bool) -> AdbResult<String> {
            let mut base_setting_cmd = vec!["settings", "put", "global", "airplane_mode_on"];
            let mut base_am_cmd = vec![
                "am",
                "broadcast",
                "-a",
                "android.intent.action.AIRPLANE_MODE",
                "--ez",
                "state",
            ];
            if status == true {
                base_setting_cmd.push("1");
                base_am_cmd.push("true");
            } else {
                base_setting_cmd.push("0");
                base_am_cmd.push("false");
            }
            self.shell(base_setting_cmd)?;
            self.shell(base_am_cmd)
        }

        pub fn switch_wifi(&mut self, status: bool) -> AdbResult<String> {
            let mut args = vec!["svc", "wifi"];
            if status == true {
                args.push("enable");
            } else {
                args.push("disable");
            };
            self.shell(args)
        }

        pub fn click(&mut self, x: i32, y: i32) -> AdbResult<String> {
            self.shell(["input", "tap", &x.to_string(), &y.to_string()])
        }

        pub fn swipe(
            &mut self,
            x1: i32,
            y1: i32,
            x2: i32,
            y2: i32,
            duration: i32,
        ) -> AdbResult<String> {
            self.shell([
                "input",
                "swipe",
                &x1.to_string(),
                &y1.to_string(),
                &x2.to_string(),
                &y2.to_string(),
                &duration.to_string(),
            ])
        }

        pub fn send_keys(&mut self, keys: &str) -> AdbResult<String> {
            self.shell(["input", "text", keys])
        }

        pub fn wlan_ip(&mut self) -> AdbResult<String> {
            let interface_commands = [
                ("wlan0", vec!["ip", "addr", "show", "dev", "wlan0"]),
                ("wlan0", vec!["ifconfig", "wlan0"]),
                ("eth0", vec!["ip", "addr", "show", "dev", "eth0"]),
                ("eth0", vec!["ifconfig", "eth0"]),
                ("", vec!["ip", "route", "get", "1.1.1.1"]),
            ];

            for (interface, cmd) in &interface_commands {
                if let Ok(result) = self.shell(cmd) {
                    if let Some(ip) = extract_ip_from_output(&result) {
                        log::info!("Found IP {} on interface {}", ip, interface);
                        return Ok(ip);
                    }
                }
            }

            Err(AdbError::from_display(
                "Failed to retrieve WLAN IP from any interface",
            ))
        }

        pub fn uninstall(&mut self, package_name: &str) -> AdbResult<String> {
            self.shell(["am", "uninstall", package_name])
        }

        pub fn app_start(&mut self, package_name: &str) -> AdbResult<String> {
            self.shell(["am", "start", "-n", package_name])
        }

        pub fn app_stop(&mut self, package_name: &str) -> AdbResult<String> {
            self.shell(["am", "force-stop", package_name])
        }

        pub fn app_clear_data(&mut self, package_name: &str) -> AdbResult<String> {
            self.shell(["pm", "clear", package_name])
        }

        pub fn app_info(&mut self, package_name: &str) -> Option<AppInfo> {
            // 首先检查应用是否存在
            let output = self.shell(["pm", "list", "packages", package_name]).ok()?;
            if !output.contains(&format!("package:{}", package_name)) {
                return None;
            }

            // 修复：dumpsys 命令拼写错误
            let app_info_output = self
                .shell(["dumpsys", "package", package_name]) // 修复：pacakge -> package
                .ok()?;

            let mut app_info = AppInfo::new(package_name);

            // 使用更健壮的正则表达式匹配
            extract_app_version_info(&app_info_output, &mut app_info);
            extract_app_signature(&app_info_output, &mut app_info);
            extract_app_flags(&app_info_output, &mut app_info);
            extract_app_timestamps(&app_info_output, &mut app_info);

            Some(app_info)
        }

        pub fn if_screen_on(&mut self) -> AdbResult<bool> {
            let resp = self.shell(["dumpsys", "power"])?;
            Ok(resp.contains("mHoldingDisplaySuspendBlocker=true"))
        }

        pub fn remove(&mut self, path: &str) -> AdbResult<String> {
            self.shell_trim(["rm", path])
        }

        pub fn get_sdk_version(&mut self) -> AdbResult<String> {
            self.shell_trim(["getprop", "ro.build.version.sdk"])
        }

        pub fn get_android_version(&mut self) -> AdbResult<String> {
            self.shell_trim(["getprop", "ro.build.version.release"])
        }

        pub fn get_device_model(&mut self) -> AdbResult<String> {
            self.shell_trim(["getprop", "ro.product.model"])
        }

        pub fn get_device_brand(&mut self) -> AdbResult<String> {
            self.shell_trim(["getprop", "ro.product.brand"])
        }
        pub fn get_device_manufacturer(&mut self) -> AdbResult<String> {
            self.shell_trim(["getprop", "ro.product.manufacturer"])
        }
        pub fn get_device_product(&mut self) -> AdbResult<String> {
            self.shell_trim(["getprop", "ro.product.product"])
        }

        pub fn get_device_abi(&mut self) -> AdbResult<String> {
            self.shell_trim(["getprop", "ro.product.cpu.abi"])
        }

        pub fn get_device_gpu(&mut self) -> AdbResult<String> {
            let resp = self.shell(["dumpsys", "SurfaceFlinger"]);
            match resp {
                Ok(data) => {
                    for x in data.split("\n") {
                        if x.starts_with("GLES:") {
                            return Ok(x.to_string());
                        }
                    }
                }
                _ => {}
            }
            Err(AdbError::from_display("fail to get gpu"))
        }
        pub fn logcat(
            &mut self,
            flush_exist: bool,
            command: Option<AdbCommand>,
        ) -> AdbResult<LogcatIterator> {
            if flush_exist {
                self.shell(["logcat", "-c"])?;
            }
            let conn = if let Some(command) = command {
                self.shell_stream(command)?
            } else {
                self.shell_stream(["logcat", "-v", "time"])?
            };
            Ok(LogcatIterator {
                reader: BufReader::new(conn),
                buffer: String::new(),
            })
        }
    }
}
