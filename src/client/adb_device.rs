use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::fs::File;
use std::{fs, thread, time};

#[cfg(feature = "blocking")]
use std::io::{BufRead, BufReader, Read, Write};
#[cfg(feature = "blocking")]
use std::net::{TcpStream, ToSocketAddrs};
#[cfg(feature = "blocking")]
use std::sync::{Arc, RwLock};

use anyhow::{anyhow, Context};
use std::path::PathBuf;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

#[cfg(feature = "tokio_async")]
use async_stream::stream;
use chrono::DateTime;

#[cfg(feature = "tokio_async")]
use futures_core::Stream;
#[cfg(feature = "tokio_async")]
use futures_util::pin_mut;
#[cfg(feature = "tokio_async")]
use futures_util::StreamExt;
#[cfg(feature = "tokio_async")]
use tokio::net::{TcpStream, ToSocketAddrs};
#[cfg(feature = "tokio_async")]
use tokio::process::Command;

use log::{error, info};

use crate::beans::file_info::{parse_file_info, FileInfo};
use crate::beans::forward_item::ForwardItem;
use crate::beans::net_info::NetworkType;

use crate::beans::app_info::AppInfo;
use crate::utils::{adb_path, get_free_port, init_logger};
use image::{io::Reader as ImageReader, RgbImage};

#[cfg(feature = "tokio_async")]
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufStream};

use crate::protocols::AdbProtocol;

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
    pub fn get_open_transport_prefix(&self, command: Option<&str>) -> anyhow::Result<String> {
        // 检查序列号和传输ID，不能同时为None
        if self.serial.is_none() & self.transport_id.is_none() {
            return Err(anyhow!(
                "TransportID and Serial Can Not Been None At Same Time"
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
            .map(|arg| {
                let mut quoted_arg = String::new();
                for c in arg.chars() {
                    if c == '"' {
                        quoted_arg.push_str("\\\"");
                    } else if c == '\\' {
                        quoted_arg.push_str("\\\\");
                    } else {
                        quoted_arg.push(c);
                    }
                }
                format!("\"{}\"", quoted_arg)
            })
            .collect::<Vec<String>>()
            .join(" ")
    }
}

#[cfg(feature = "tokio_async")]
impl<T> AdbDevice<T>
where
    T: ToSocketAddrs + Clone,
{
    pub async fn open_transport(&mut self, command: Option<&str>) -> anyhow::Result<TcpStream> {
        // 获取打开传输的前缀，基于是否提供了命令和设备的序列号或传输ID。
        let prefix = self
            .get_open_transport_prefix(command)
            .map_err(|_| anyhow!("Get Open Transport Prefix Failed"))?;
        let mut stream = TcpStream::connect(self.addr.clone()).await?;
        stream
            .send_cmd_then_check_okay(&prefix)
            .await
            .context(format!(
                "Send Command >> {:#?} and Check Okay Failed",
                &prefix
            ))?;
        Ok(stream)
    }

    async fn get_with_command(&mut self, command: &str) -> anyhow::Result<String> {
        let mut conn = self.open_transport(Some(command)).await?;
        let result = conn.read_string_block().await?;
        Ok(result)
    }

    ///
    /// 与 命令 adb get-state 相同  => device
    pub async fn get_state(&mut self) -> anyhow::Result<String> {
        self.get_with_command("get-state").await
    }

    ///
    /// adb get-serialno => emulator-5554
    pub async fn get_serialno(&mut self) -> anyhow::Result<String> {
        self.get_with_command("get-serialno").await
    }

    ///adb get-devpath
    pub async fn get_devpath(&mut self) -> anyhow::Result<String> {
        self.get_with_command("get-devpath").await
    }

    pub async fn get_features(&mut self) -> anyhow::Result<String> {
        self.get_with_command("get-features").await
    }

    /// 执行通过ADB shell命令流，并返回一个AdbConnection的实例。
    ///
    /// # 参数
    /// - `command`: 一个包含多个命令参数的字符串切片数组，每个元素都是一个命令参数。
    ///
    /// # 返回值
    /// - `anyhow::Result<AdbConnection>`: 如果命令成功执行，则返回一个AdbConnection的实例；
    ///                                  如果执行过程中出现错误，则返回错误信息。
    pub async fn shell_stream(&mut self, command: &[&str]) -> anyhow::Result<TcpStream> {
        // 打开与设备的传输通道
        let mut conn = self.open_transport(None).await?;

        // 将命令切片数组转换为命令行字符串
        let cmd = Self::list2cmdline(command);

        // 构造完整的ADB shell命令字符串
        let send_cmd = format!("shell:{}", cmd);

        // 发送命令并检查是否执行成功
        let _ = conn
            .send_cmd_then_check_okay(&send_cmd)
            .await
            .context(format!(
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
    /// - `anyhow::Result<String>`: 命令执行成功则返回命令的输出结果，如果执行过程中出现错误则返回错误信息。
    pub async fn shell(&mut self, command: &[&str]) -> anyhow::Result<String> {
        // 通过`shell_stream`方法执行命令，获取命令的输出流
        let mut s = self.shell_stream(command).await?;

        // 从输出流中读取直到流关闭的所有数据，并将其存储为字符串
        let output = s.read_until_close().await?;

        // 将读取到的命令输出返回
        Ok(output)
    }

    pub async fn shell_trim(&mut self, command: &[&str]) -> anyhow::Result<String> {
        let s = self.shell(command).await?;
        Ok(s.trim().to_string())
    }

    pub async fn forward(
        &mut self,
        local: &str,
        remote: &str,
        norebind: bool,
    ) -> anyhow::Result<()> {
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
        Err(anyhow!("Failed To Forward Port"))
    }

    pub async fn forward_list(&mut self) -> anyhow::Result<Vec<ForwardItem>> {
        let mut connection = self.open_transport(Some("list-forward")).await?;
        let content = connection.read_string_block().await?;
        let objs = content
            .lines()
            .map(|x| {
                let mut s = x.split(" ");
                if let (3, Some(3)) = s.size_hint() {
                    let (serial, local, remote) = (
                        s.nth(0).unwrap_or(""),
                        s.nth(1).unwrap_or(""),
                        s.nth(2).unwrap_or(""),
                    );
                    Some(ForwardItem::new(serial, local, remote))
                } else {
                    None
                }
            })
            .filter(|x| x.is_some())
            .map(|x| x.unwrap())
            .collect();
        Ok(objs)
    }
    pub async fn forward_remote_port(&mut self, remote: u16) -> anyhow::Result<u16> {
        let remote = format!("tcp:{}", remote);
        let local_port = get_free_port()?;
        let local = format!("tcp:{}", local_port);
        match self.forward(&local, &remote, false).await {
            Ok(_) => Ok(local_port),
            Err(e) => Err(anyhow!("Failed To Forward Port, Err >>> {}", e)),
        }
    }
    pub async fn reverse(
        &mut self,
        remote: &str,
        local: &str,
        norebind: bool,
    ) -> anyhow::Result<()> {
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
    ) -> anyhow::Result<TcpStream> {
        let mut connection = self.open_transport(None).await?;
        let cmd = match network_type {
            NetworkType::LocalAbstrcat | NetworkType::Unix => {
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
    pub async fn adb_output(&mut self, command: &[&str]) -> anyhow::Result<String> {
        let adb_ = adb_path()?;
        if adb_.exists() {
            if let Some(ref serial) = self.serial {
                let cmd = Command::new(adb_)
                    .arg("-s")
                    .arg(serial)
                    .args(command)
                    .output()
                    .await?;
                return Ok(String::from_utf8_lossy(&cmd.stdout).parse()?);
            }
        };
        Err(anyhow!("adb not found"))
    }

    pub async fn tcpip(&mut self, port: u16) -> anyhow::Result<String> {
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

    pub async fn push(&mut self, local: &str, remote: &str) -> anyhow::Result<()> {
        if self.adb_output(&["push", local, remote]).await.is_ok() {
            info!("push {} to {} success", local, remote);
            return Ok(());
        }
        Err(anyhow!("push error"))
    }
    pub async fn pull(&mut self, src: &str, dest: &PathBuf) -> anyhow::Result<usize> {
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
    ) -> anyhow::Result<impl Stream<Item = anyhow::Result<(Vec<u8>, String)>>> {
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

    pub async fn exists(&mut self, path: &str) -> anyhow::Result<bool> {
        let file_info = self.stat(path).await?;
        if file_info.mtime != 0 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn stat(&mut self, path: &str) -> anyhow::Result<FileInfo> {
        let mut conn = self.prepare_sync(path, "STAT").await?;
        let data = conn.read_string(4).await?;
        if data.eq("STAT") {
            let current_data = conn.recv(12).await?;
            return Ok(parse_file_info(current_data, path)?);
        };
        Err(anyhow!("stat error"))
    }

    pub async fn list(&mut self, path: &str) -> anyhow::Result<Vec<FileInfo>> {
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
    ) -> anyhow::Result<impl Stream<Item = anyhow::Result<String>>> {
        let mut stream = self.iter_content(path).await?;
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

    pub async fn prepare_sync(&mut self, path: &str, command: &str) -> anyhow::Result<TcpStream> {
        info!("Start Sync Path {:#?} With Command {:#?}", path, command);
        let mut conn = self.open_transport(None).await?;
        conn.send_cmd_then_check_okay("sync:")
            .await
            .context("Start Sync Error")?;
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
    ) -> anyhow::Result<impl Stream<Item = anyhow::Result<Vec<u8>>>> {
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

    pub async fn screenshot(&mut self) -> anyhow::Result<RgbImage> {
        let src = "/sdcard/screen.png";
        self.shell(&["screencap", "-p", src]).await?;
        let tmpdir = tempfile::tempdir().expect("Failed to create temporary directory");
        let target_path = tmpdir.path().join("tmp001.png");
        info!("Pull Image To {:#?}", &target_path);
        self.pull(src, &target_path).await?;
        self.shell(&["rm", src]).await?;

        let image = ImageReader::open(&target_path)?.decode()?;
        fs::remove_file(target_path).expect("Failed to remove file");
        Ok(image.into_rgb8())
    }

    pub async fn keyevent(&mut self, keycode: &str) -> anyhow::Result<String> {
        self.shell(&["input", "keyevent", keycode]).await
    }

    pub async fn switch_screen(&mut self, status: bool) -> anyhow::Result<String> {
        if status == true {
            self.keyevent("224").await
        } else {
            self.keyevent("223").await
        }
    }

    pub async fn install(&mut self, path_or_url: &str) -> anyhow::Result<(), anyhow::Error> {
        let target_path =
            if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
                let mut resp = reqwest::get(path_or_url).await?;
                let response_bytes = resp.bytes().await?;
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
                Err(anyhow!(e))
            }
        }
    }
    pub async fn install_remote(&mut self, path: &str, clean: bool) -> anyhow::Result<String> {
        let args = ["pm", "install", "-r", "-t", path];
        let output = self.shell(&args).await?;
        if !output.contains("Success") {
            return Err(anyhow!("fail to install"));
        };
        if clean {
            self.shell(&["rm", path]).await?;
        }
        Ok(output)
    }

    pub async fn switch_airplane_mode(&mut self, status: bool) -> anyhow::Result<String> {
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
        self.shell(&base_setting_cmd).await?;
        self.shell(&base_am_cmd).await
    }

    pub async fn switch_wifi(&mut self, status: bool) -> anyhow::Result<String> {
        let mut args = vec!["svc", "wifi"];
        if status == true {
            args.push("enable");
        } else {
            args.push("disable");
        };
        self.shell(&args).await
    }

    pub async fn click(&mut self, x: i32, y: i32) -> anyhow::Result<String> {
        self.shell(&["input", "tap", &x.to_string(), &y.to_string()])
            .await
    }

    pub async fn swipe(
        &mut self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        duration: i32,
    ) -> anyhow::Result<String> {
        self.shell(&[
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

    pub async fn send_keys(&mut self, keys: &str) -> anyhow::Result<String> {
        self.shell(&["input", "text", keys]).await
    }

    pub async fn wlan_ip(&mut self) -> anyhow::Result<String> {
        let mut result = self.shell(&["ifconfig", "wlan0"]).await?;
        let re = regex::Regex::new(r"inet\s*addr:(.*?)\s").unwrap();
        if let Some(captures) = re.captures(&result) {
            return Ok(captures.get(1).unwrap().as_str().to_string());
        }
        result = self.shell(&["ip", "addr", "show", "dev", "wlan0"]).await?;
        let re = regex::Regex::new(r"inet (\d+.*?)/\d+").unwrap();
        if let Some(captures) = re.captures(&result) {
            return Ok(captures.get(1).unwrap().as_str().to_string());
        }

        result = self.shell(&["ifconfig", "eth0"]).await?;
        let re = regex::Regex::new(r"inet\s*addr:(.*?)\s").unwrap();
        if let Some(captures) = re.captures(&result) {
            return Ok(captures.get(1).unwrap().as_str().to_string());
        }
        Err(anyhow!("fail to parse wlan ip"))
    }

    pub async fn uninstall(&mut self, package_name: &str) -> anyhow::Result<String> {
        self.shell(&["am", "uninstall", package_name]).await
    }

    pub async fn app_start(&mut self, package_name: &str) -> anyhow::Result<String> {
        self.shell(&["am", "start", "-n", package_name]).await
    }

    pub async fn app_stop(&mut self, package_name: &str) -> anyhow::Result<String> {
        self.shell(&["am", "force-stop", package_name]).await
    }

    pub async fn app_clear_data(&mut self, package_name: &str) -> anyhow::Result<String> {
        self.shell(&["pm", "clear", package_name]).await
    }

    pub async fn app_info(&mut self, package_name: &str) -> Option<AppInfo> {
        let output = self.shell(&["pm", "list", "package", "-3"]).await.ok()?;
        if !output.contains(&format!("package:{}", package_name)) {
            return None;
        }
        let app_info_output = self
            .shell(&["dumpsys", "pacakge", package_name])
            .await
            .ok()?;
        let mut app_info = AppInfo::new(package_name);
        let version_name_regex = regex::Regex::new(r"versionName=(?P<name>\S+)").unwrap();
        if let Some(cap) = version_name_regex.captures(&app_info_output) {
            let version_name = cap.get(1).unwrap().as_str();
            app_info.version_name = Some(version_name.to_string());
        }
        let version_code_regex = regex::Regex::new(r"versionCode=(?P<code>\d+)").unwrap();
        if let Some(cap) = version_code_regex.captures(&app_info_output) {
            let version_code = cap.get(1).unwrap().as_str();
            app_info.version_code = Some(u32::from_str(version_code).ok()?);
        }
        let package_signature_regex = regex::Regex::new(r"PackageSignatures\{.*?\[(.*)]}").unwrap();
        if let Some(cap) = package_signature_regex.captures(&app_info_output) {
            let signature = cap.get(1).unwrap().as_str();
            app_info.signature = Some(signature.to_string());
        }

        if app_info.version_code.as_ref().is_none() && app_info.version_name.as_ref().is_none() {
            return Some(app_info);
        }
        let pkg_flags_regex = regex::Regex::new(r"pkgFlags=\[\s*(.*)\s*]").unwrap();
        let mut flags = vec![];
        for (_, [flag]) in pkg_flags_regex
            .captures_iter(&app_info_output)
            .map(|c| c.extract())
        {
            flags.push(flag.to_string())
        }
        app_info.flags = flags;

        let first_install_time_regex =
            regex::Regex::new(r"firstInstallTime=(?P<time>[-\d]+\s+[:\d]+)").unwrap();
        if let Some(cap) = first_install_time_regex.captures(&app_info_output) {
            let first_install_time = cap.get(1).unwrap().as_str();
            app_info.first_install_time = Some(DateTime::from_str(first_install_time).ok()?);
        }
        let last_update_time_regex =
            regex::Regex::new(r"lastUpdateTime=(?P<time>[-\d]+\s+[:\d]+)").unwrap();
        if let Some(cap) = last_update_time_regex.captures(&app_info_output) {
            let first_install_time = cap.get(1).unwrap().as_str();
            app_info.last_update_time = Some(DateTime::from_str(first_install_time).ok()?);
        }
        Some(app_info)
    }

    pub async fn if_screen_on(&mut self) -> anyhow::Result<bool> {
        let resp = self.shell(&["dumpsys", "power"]).await?;
        Ok(resp.contains("mHoldingDisplaySuspendBlocker=true"))
    }

    pub async fn remove(&mut self, path: &str) -> anyhow::Result<String> {
        self.shell_trim(&["rm", path]).await
    }

    pub async fn get_sdk_version(&mut self) -> anyhow::Result<String> {
        self.shell_trim(&["getprop", "ro.build.version.sdk"]).await
    }

    pub async fn get_android_version(&mut self) -> anyhow::Result<String> {
        self.shell_trim(&["getprop", "ro.build.version.release"])
            .await
    }

    pub async fn get_device_model(&mut self) -> anyhow::Result<String> {
        self.shell_trim(&["getprop", "ro.product.model"]).await
    }

    pub async fn get_device_brand(&mut self) -> anyhow::Result<String> {
        self.shell_trim(&["getprop", "ro.product.brand"]).await
    }
    pub async fn get_device_manufacturer(&mut self) -> anyhow::Result<String> {
        self.shell_trim(&["getprop", "ro.product.manufacturer"])
            .await
    }
    pub async fn get_device_product(&mut self) -> anyhow::Result<String> {
        self.shell_trim(&["getprop", "ro.product.product"]).await
    }

    pub async fn get_device_abi(&mut self) -> anyhow::Result<String> {
        self.shell_trim(&["getprop", "ro.product.cpu.abi"]).await
    }

    pub async fn get_device_gpu(&mut self) -> anyhow::Result<String> {
        let resp = self.shell(&["dumpsys", "SurfaceFlinger"]).await;
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
        Err(anyhow!("fail to get gpu"))
    }
    pub async fn logcat(
        &mut self,
        flush_exist: bool,
        extra_command: Option<&[&str]>,
    ) -> anyhow::Result<impl Stream<Item = anyhow::Result<String>>> {
        if flush_exist {
            self.shell(&["logcat", "-c"]).await?;
        };
        let cmd = if let Some(extra_cmd) = extra_command {
            let mut default_cmd = vec!["logcat"];
            default_cmd.extend_from_slice(extra_cmd);
            default_cmd
        } else {
            vec!["logcat", "-v", "time"]
        };
        let conn = self.shell_stream(&cmd).await?;
        Ok(stream! {
                    let mut reader = BufStream::new(conn);
                    let mut buffer = String::new();
                    loop {
                        buffer.clear();
                        match reader.read_line(&mut buffer).await {
                        Ok(0) => break,
                        Ok(_) => {
                            yield Ok(buffer.clone());
                        }
                        Err(e) => {
                            yield Err(anyhow!(e));  // 出错时发出错误
                            break;
                            }
                        }
                    }
        })
    }
}

#[cfg(feature = "blocking")]
impl<T> AdbDevice<T>
where
    T: ToSocketAddrs + Clone + Debug,
{
    /// 打开一个Adb连接，通过给定的命令选项配置传输前缀。
    ///
    /// - `command`：可选的命令字符串，用于配置传输前缀。
    /// - 返回值：成功时返回一个`AdbConnection`实例，表示与设备的连接。
    pub fn open_transport(&mut self, command: Option<&str>) -> anyhow::Result<TcpStream> {
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

    fn get_with_command(&mut self, command: &str) -> anyhow::Result<String> {
        let mut conn = self.open_transport(Some(command))?;
        let result = conn.read_string_block()?;
        Ok(result)
    }

    pub fn get_state(&mut self) -> anyhow::Result<String> {
        self.get_with_command("get-state")
    }

    pub fn get_serialno(&mut self) -> anyhow::Result<String> {
        self.get_with_command("get-serialno")
    }

    pub fn get_devpath(&mut self) -> anyhow::Result<String> {
        self.get_with_command("get-devpath")
    }

    pub fn get_features(&mut self) -> anyhow::Result<String> {
        self.get_with_command("get-features")
    }

    /// 执行通过ADB shell命令流，并返回一个AdbConnection的实例。
    ///
    /// # 参数
    /// - `command`: 一个包含多个命令参数的字符串切片数组，每个元素都是一个命令参数。
    ///
    /// # 返回值
    /// - `anyhow::Result<AdbConnection>`: 如果命令成功执行，则返回一个AdbConnection的实例；
    ///                                  如果执行过程中出现错误，则返回错误信息。
    pub fn shell_stream(&mut self, command: &[&str]) -> anyhow::Result<TcpStream> {
        // 打开与设备的传输通道
        let mut conn = self.open_transport(None)?;

        // 将命令切片数组转换为命令行字符串
        let cmd = Self::list2cmdline(command);

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
    /// - `anyhow::Result<String>`: 命令执行成功则返回命令的输出结果，如果执行过程中出现错误则返回错误信息。
    pub fn shell(&mut self, command: &[&str]) -> anyhow::Result<String> {
        // 通过`shell_stream`方法执行命令，获取命令的输出流
        let mut s = self.shell_stream(command)?;

        // 从输出流中读取直到流关闭的所有数据，并将其存储为字符串
        let output = s.read_until_close()?;

        // 将读取到的命令输出返回
        Ok(output)
    }
    pub fn shell_trim(&mut self, command: &[&str]) -> anyhow::Result<String> {
        let mut s = self.shell_stream(command)?;
        let output = s.read_until_close()?;
        Ok(output.trim().to_string())
    }

    pub fn forward(&mut self, local: &str, remote: &str, norebind: bool) -> anyhow::Result<()> {
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
        Err(anyhow!("Failed To Forward Port"))
    }

    pub fn forward_list(&mut self) -> anyhow::Result<Vec<ForwardItem>> {
        let mut connection = self.open_transport(Some("list-forward"))?;
        let content = connection.read_string_block()?;
        let mut forward_iterms = vec![];
        for x in content.lines() {
            let mut current_parts: Vec<&str> = x.split(" ").collect();
            if current_parts.len() == 3 {
                let (serial, local, remote) =
                    (current_parts[0], current_parts[1], current_parts[2]);
                forward_iterms.push(ForwardItem::new(serial, local, remote))
            }
        }
        Ok(forward_iterms)
    }
    pub fn forward_remote_port(&mut self, remote: u16) -> anyhow::Result<u16> {
        let remote = format!("tcp:{}", remote);
        for x in self.forward_list()? {
            if x.serial.eq(self.serial.clone().unwrap().as_str())
                & x.remote.eq(&remote)
                & x.local.starts_with("tcp:")
            {
                u16::from_str(x.local.split("tcp:").last().unwrap()).unwrap();
            }
        }
        let local_port = get_free_port()?;
        let local = format!("tcp:{}", local_port);
        match self.forward(&local, &remote, false) {
            Ok(_) => Ok(local_port),
            Err(_) => Err(anyhow!("Failed To Forward Port")),
        }
    }

    pub fn reverse(&mut self, remote: &str, local: &str, norebind: bool) -> anyhow::Result<()> {
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

    pub fn adb_output(&mut self, command: &[&str]) -> anyhow::Result<String> {
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
            return Ok(String::from_utf8_lossy(&output.stdout).parse()?);
        };
        Err(anyhow!("adb not found"))
    }

    pub fn tcpip(&mut self, port: u16) -> anyhow::Result<String> {
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
    pub fn push(&mut self, local: &str, remote: &str) -> anyhow::Result<()> {
        if self.adb_output(&["push", local, remote]).is_ok() {
            info!("push {} to {} success", local, remote);
            return Ok(());
        }
        Err(anyhow!("push error"))
    }
    pub fn pull(&mut self, src: &str, dest: &PathBuf) -> anyhow::Result<usize> {
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

    pub fn iter_directory(&mut self, path: &str) -> anyhow::Result<impl Iterator<Item = FileInfo>> {
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

    pub fn exists(&mut self, path: &str) -> anyhow::Result<bool> {
        let file_info = self.stat(path)?;
        if file_info.mtime != 0 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn stat(&mut self, path: &str) -> anyhow::Result<FileInfo> {
        let mut conn = self.prepare_sync(path, "STAT")?;
        let data = conn.read_string(4)?;
        if data.eq("STAT") {
            let current_data = conn.recv(12)?;
            return Ok(parse_file_info(current_data, path)?);
        };
        Err(anyhow!("stat error"))
    }

    pub fn list(&mut self, path: &str) -> anyhow::Result<Vec<FileInfo>> {
        Ok(self
            .iter_directory(path)
            .context("Iter Directory Error")?
            .collect::<Vec<FileInfo>>())
    }

    pub fn read_text(&mut self, path: &str) -> anyhow::Result<String> {
        let data = self
            .iter_content(path)?
            .map(|x| x.unwrap_or_else(|_| "".to_string()))
            .collect::<Vec<String>>();
        Ok(data.join(""))
    }

    pub fn prepare_sync(&mut self, path: &str, command: &str) -> anyhow::Result<TcpStream> {
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
    ) -> anyhow::Result<impl Iterator<Item = anyhow::Result<String>>> {
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
                                let str_size = u32::from_le_bytes(data.try_into().ok()?) as usize;
                                let error_message = connection.read_string(str_size).ok()?;
                                error!("Sync Error With Error Message >>> {:#?}", error_message);
                                None
                            }
                        },
                        "DONE" => {
                            done = true;
                            None
                        }
                        "DATA" => match connection.recv(4) {
                            Ok(size) => {
                                let str_size = u32::from_le_bytes(size.try_into().ok()?) as usize;
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
        Err(anyhow!("iter_content error"))
    }

    pub fn screenshot(&mut self) -> anyhow::Result<RgbImage> {
        let src = "/sdcard/screen.png";
        self.shell(&["screencap", "-p", src])?;
        let tmpdir = tempfile::tempdir().expect("Failed to create temporary directory");
        let target_path = tmpdir.path().join("tmp001.png");
        info!("Pull Image To {:#?}", &target_path);
        self.pull(src, &target_path)?;
        self.shell(&["rm", src])?;

        let image = ImageReader::open(&target_path)?.decode()?;
        fs::remove_file(target_path).expect("Failed to remove file");
        Ok(image.into_rgb8())
    }

    pub fn keyevent(&mut self, keycode: &str) -> anyhow::Result<String> {
        self.shell(&["input", "keyevent", keycode])
    }

    pub fn switch_screen(&mut self, status: bool) -> anyhow::Result<String> {
        if status == true {
            self.keyevent("224")
        } else {
            self.keyevent("223")
        }
    }

    pub fn install(&mut self, path_or_url: &str) -> anyhow::Result<(), anyhow::Error> {
        let target_path =
            if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
                let mut resp = reqwest::blocking::get(path_or_url)?;
                let mut buffer = Vec::new();
                resp.read_to_end(&mut buffer)?;
                let temp_dir = tempfile::tempdir()?.path().join("tmp001.apk");
                let mut fd = File::create(&temp_dir)?;
                fd.write_all(&buffer)?;
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
        self.push(&target_path, &dst)?;
        let install_resp = self.install_remote(&dst, true);
        info!("Install Apk Result {:#?}", &install_resp);
        if let Ok(resp) = install_resp {
            info!("Install Apk Successed >> <{:#?}>", &resp);
            return Ok(());
        }
        Err(anyhow!("fail to install apk"))
    }
    pub fn install_remote(&mut self, path: &str, clean: bool) -> anyhow::Result<String> {
        let args = ["pm", "install", "-r", "-t", path];
        let output = self.shell(&args)?;
        if !output.contains("Success") {
            return Err(anyhow!("fail to install"));
        };
        if clean {
            self.shell(&["rm", path])?;
        }
        Ok(output)
    }

    pub fn switch_airplane_mode(&mut self, status: bool) -> anyhow::Result<String> {
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
        self.shell(&base_setting_cmd)?;
        self.shell(&base_am_cmd)
    }

    pub fn switch_wifi(&mut self, status: bool) -> anyhow::Result<String> {
        let mut args = vec!["svc", "wifi"];
        if status == true {
            args.push("enable");
        } else {
            args.push("disable");
        };
        self.shell(&args)
    }

    pub fn click(&mut self, x: i32, y: i32) -> anyhow::Result<String> {
        self.shell(&["input", "tap", &x.to_string(), &y.to_string()])
    }

    pub fn swipe(
        &mut self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        duration: i32,
    ) -> anyhow::Result<String> {
        self.shell(&[
            "input",
            "swipe",
            &x1.to_string(),
            &y1.to_string(),
            &x2.to_string(),
            &y2.to_string(),
            &duration.to_string(),
        ])
    }

    pub fn send_keys(&mut self, keys: &str) -> anyhow::Result<String> {
        self.shell(&["input", "text", keys])
    }

    pub fn wlan_ip(&mut self) -> anyhow::Result<String> {
        let mut result = self.shell(&["ifconfig", "wlan0"])?;
        let re = regex::Regex::new(r"inet\s*addr:(.*?)\s").unwrap();
        if let Some(captures) = re.captures(&result) {
            return Ok(captures.get(1).unwrap().as_str().to_string());
        }
        result = self.shell(&["ip", "addr", "show", "dev", "wlan0"])?;
        let re = regex::Regex::new(r"inet (\d+.*?)/\d+").unwrap();
        if let Some(captures) = re.captures(&result) {
            return Ok(captures.get(1).unwrap().as_str().to_string());
        }

        result = self.shell(&["ifconfig", "eth0"])?;
        let re = regex::Regex::new(r"inet\s*addr:(.*?)\s").unwrap();
        if let Some(captures) = re.captures(&result) {
            return Ok(captures.get(1).unwrap().as_str().to_string());
        }
        Err(anyhow!("fail to parse wlan ip"))
    }

    pub fn uninstall(&mut self, package_name: &str) -> anyhow::Result<String> {
        self.shell(&["am", "uninstall", package_name])
    }

    pub fn app_start(&mut self, package_name: &str) -> anyhow::Result<String> {
        self.shell(&["am", "start", "-n", package_name])
    }

    pub fn app_stop(&mut self, package_name: &str) -> anyhow::Result<String> {
        self.shell(&["am", "force-stop", package_name])
    }

    pub fn app_clear_data(&mut self, package_name: &str) -> anyhow::Result<String> {
        self.shell(&["pm", "clear", package_name])
    }

    pub fn app_info(&mut self, package_name: &str) -> Option<AppInfo> {
        let output = self.shell(&["pm", "list", "package", "-3"]).ok()?;
        if !output.contains(&format!("package:{}", package_name)) {
            return None;
        }
        let app_info_output = self.shell(&["dumpsys", "pacakge", package_name]).ok()?;
        let mut app_info = AppInfo::new(package_name);
        let version_name_regex = regex::Regex::new(r"versionName=(?P<name>\S+)").unwrap();
        if let Some(cap) = version_name_regex.captures(&app_info_output) {
            let version_name = cap.get(1).unwrap().as_str();
            app_info.version_name = Some(version_name.to_string());
        }
        let version_code_regex = regex::Regex::new(r"versionCode=(?P<code>\d+)").unwrap();
        if let Some(cap) = version_code_regex.captures(&app_info_output) {
            let version_code = cap.get(1).unwrap().as_str();
            app_info.version_code = Some(u32::from_str(version_code).ok()?);
        }
        let package_signature_regex = regex::Regex::new(r"PackageSignatures\{.*?\[(.*)]}").unwrap();
        if let Some(cap) = package_signature_regex.captures(&app_info_output) {
            let signature = cap.get(1).unwrap().as_str();
            app_info.signature = Some(signature.to_string());
        }

        if app_info.version_code.as_ref().is_none() && app_info.version_name.as_ref().is_none() {
            return Some(app_info);
        }
        let pkg_flags_regex = regex::Regex::new(r"pkgFlags=\[\s*(.*)\s*]").unwrap();
        let mut flags = vec![];
        for (_, [flag]) in pkg_flags_regex
            .captures_iter(&app_info_output)
            .map(|c| c.extract())
        {
            flags.push(flag.to_string())
        }
        app_info.flags = flags;

        let first_install_time_regex =
            regex::Regex::new(r"firstInstallTime=(?P<time>[-\d]+\s+[:\d]+)").unwrap();
        if let Some(cap) = first_install_time_regex.captures(&app_info_output) {
            let first_install_time = cap.get(1).unwrap().as_str();
            app_info.first_install_time = Some(DateTime::from_str(first_install_time).ok()?);
        }
        let last_update_time_regex =
            regex::Regex::new(r"lastUpdateTime=(?P<time>[-\d]+\s+[:\d]+)").unwrap();
        if let Some(cap) = last_update_time_regex.captures(&app_info_output) {
            let first_install_time = cap.get(1).unwrap().as_str();
            app_info.last_update_time = Some(DateTime::from_str(first_install_time).ok()?);
        }
        Some(app_info)
    }

    pub fn if_screen_on(&mut self) -> anyhow::Result<bool> {
        let resp = self.shell(&["dumpsys", "power"])?;
        Ok(resp.contains("mHoldingDisplaySuspendBlocker=true"))
    }

    pub fn remove(&mut self, path: &str) -> anyhow::Result<String> {
        self.shell_trim(&["rm", path])
    }

    pub fn get_sdk_version(&mut self) -> anyhow::Result<String> {
        self.shell_trim(&["getprop", "ro.build.version.sdk"])
    }

    pub fn get_android_version(&mut self) -> anyhow::Result<String> {
        self.shell_trim(&["getprop", "ro.build.version.release"])
    }

    pub fn get_device_model(&mut self) -> anyhow::Result<String> {
        self.shell_trim(&["getprop", "ro.product.model"])
    }

    pub fn get_device_brand(&mut self) -> anyhow::Result<String> {
        self.shell_trim(&["getprop", "ro.product.brand"])
    }
    pub fn get_device_manufacturer(&mut self) -> anyhow::Result<String> {
        self.shell_trim(&["getprop", "ro.product.manufacturer"])
    }
    pub fn get_device_product(&mut self) -> anyhow::Result<String> {
        self.shell_trim(&["getprop", "ro.product.product"])
    }

    pub fn get_device_abi(&mut self) -> anyhow::Result<String> {
        self.shell_trim(&["getprop", "ro.product.cpu.abi"])
    }

    pub fn get_device_gpu(&mut self) -> anyhow::Result<String> {
        let resp = self.shell(&["dumpsys", "SurfaceFlinger"]);
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
        Err(anyhow!("fail to get gpu"))
    }
    pub fn logcat(
        &mut self,
        flush_exist: bool,
        command: Option<&str>,
        lock: Arc<RwLock<bool>>,
    ) -> anyhow::Result<impl Iterator<Item = String>> {
        if flush_exist {
            self.shell(&["logcat", "-c"])?;
        }
        let mut conn = self.shell_stream(&["logcat", "-v", "time"])?;
        Ok(std::iter::from_fn(move || {
            let mut bufreader = BufReader::new(&conn);

            while *(lock.read().unwrap()) {
                let mut string = String::new();
                let data = bufreader.read_line(&mut string);
                if data.is_ok() {
                    return Some(string);
                }
            }
            None
        }))
    }
}
