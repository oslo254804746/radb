use std::collections::HashMap;
use std::fmt::Debug;

use crate::client::common;
use crate::errors::AdbResult;
#[cfg(feature = "blocking")]
use std::net::ToSocketAddrs;
#[cfg(feature = "tokio_async")]
use tokio::net::ToSocketAddrs;

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
        common::build_transport_prefix(self.serial.as_deref(), self.transport_id, command)
    }

    pub fn list2cmdline(args: &[&str]) -> String {
        args.iter()
            .map(|&arg| common::escape_shell_arg(arg))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(feature = "tokio_async")]
pub mod async_impl {
    use crate::beans::command::AdbCommand;
    use crate::beans::{parse_file_info, AppInfo, FileInfo, ForwardItem, NetworkType};
    use crate::client::common::{
        self, DeviceTextCommand, build_forward_command, build_reverse_command,
        build_uninstall_command, command_output_to_result, extract_forward_item_from_output,
        extract_ip_from_output, extract_port_from_tcp_spec,
    };
    use crate::client::AdbDevice;
    use crate::errors::{AdbError, AdbResult};
    use crate::protocols::AdbProtocol;
    use crate::sync_protocol::{self, RecvFrame};
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
    use tokio::io::{AsyncBufReadExt, BufReader};
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

        async fn run_text_command(&mut self, command: DeviceTextCommand) -> AdbResult<String> {
            match command.transport_command() {
                Some(cmd) => self.get_with_command(cmd).await,
                None => {
                    let out = self.shell(command.shell_args().unwrap()).await?;
                    if command.trim_output() {
                        Ok(out.trim().to_string())
                    } else {
                        Ok(out)
                    }
                }
            }
        }

        ///
        /// 与 命令 adb get-state 相同  => device
        pub async fn get_state(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::State).await
        }

        ///
        /// adb get-serialno => emulator-5554
        pub async fn get_serialno(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::SerialNo).await
        }

        ///adb get-devpath
        pub async fn get_devpath(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::DevPath).await
        }

        pub async fn get_features(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::Features).await
        }

        /// 执行通过ADB shell命令流，并返回一个AdbConnection的实例。
        ///
        /// # 参数
        /// - `command`: 一个包含多个命令参数的字符串切片数组，每个元素都是一个命令参数。
        ///
        /// # 返回值
        /// - `AdbResult<AdbConnection>`: 如果命令成功执行，则返回一个AdbConnection的实例；否则返回错误信息。
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
            conn.send_cmd_then_check_okay(&send_cmd).await?;

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
            let full_cmd = build_forward_command(local, remote, norebind);
            self.open_transport(Some(&full_cmd)).await?;
            Ok(())
        }

        pub async fn forward_list(&mut self) -> AdbResult<Vec<ForwardItem>> {
            let mut connection = self.open_transport(Some("list-forward")).await?;
            let content = connection.read_response().await?;
            Ok(extract_forward_item_from_output(&content))
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
            let full_cmd = build_reverse_command(remote, local, norebind);
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
                _ => format!("{}{}", network_type, address),
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
                    return command_output_to_result(command, cmd);
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
            self.adb_output(&["push", local, remote]).await?;
            info!("push {} to {} success", local, remote);
            Ok(())
        }

        pub async fn pull(&mut self, src: &str, dest: &PathBuf) -> AdbResult<usize> {
            let mut size = 0;
            let mut file = File::create(dest)?;
            let stream = self.iter_content(src).await?;
            pin_mut!(stream);
            while let Some(chunk) = stream.next().await {
                let data = chunk?;
                file.write_all(&data)?;
                size += data.len();
            }
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
                            let current_data = conn.recv(16).await?;
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
            let stream = self.iter_directory(path).await?;
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
        ) -> AdbResult<impl Stream<Item = AdbResult<String>>> {
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
            let command_id: [u8; 4] = command
                .as_bytes()
                .try_into()
                .map_err(|_| AdbError::protocol_error("sync command must be 4 bytes"))?;
            let total_byte = sync_protocol::encode_path_command(&command_id, path);
            conn.send_all(&total_byte).await?;
            Ok(conn)
        }

        pub async fn iter_content(
            &mut self,
            path: &str,
        ) -> AdbResult<impl Stream<Item = AdbResult<Vec<u8>>>> {
            let mut connection = self.prepare_sync(path, "RECV").await?;
            Ok(stream! {
                loop {
                    let id = match connection.recv_exact(4).await {
                        Ok(data) => match data.try_into() {
                            Ok(id) => id,
                            Err(_) => {
                                yield Err(AdbError::protocol_error("Invalid sync frame id"));
                                break;
                            }
                        },
                        Err(e) => {
                            yield Err(e);
                            break;
                        }
                    };
                    let payload_len = match connection.recv_exact(4).await {
                        Ok(data) => match sync_protocol::parse_u32_le(&data) {
                            Ok(size) => size as usize,
                            Err(e) => {
                                yield Err(e);
                                break;
                            }
                        },
                        Err(e) => {
                            yield Err(e);
                            break;
                        }
                    };
                    let payload = match connection.recv_exact(payload_len).await {
                        Ok(payload) => payload,
                        Err(e) => {
                            yield Err(e);
                            break;
                        }
                    };

                    match sync_protocol::parse_recv_frame(id, payload) {
                        Ok(RecvFrame::Data(data)) => yield Ok(data),
                        Ok(RecvFrame::Done) => break,
                        Ok(RecvFrame::Fail(message)) => {
                            yield Err(AdbError::protocol_error(message));
                            break;
                        }
                        Err(e) => {
                            yield Err(e);
                            break;
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
            if status {
                self.keyevent("224").await
            } else {
                self.keyevent("223").await
            }
        }

        pub async fn install(&mut self, path_or_url: &str) -> AdbResult<()> {
            let _temp_dir_guard;
            let target_path;
            if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
                let resp = reqwest::get(path_or_url)
                    .await
                    .context("Fail to get http response")?;
                let response_bytes = resp.bytes().await.context("Fail to get bytes")?;
                let temp_dir = tempfile::tempdir()?;
                let temp_path = temp_dir.path().join("tmp001.apk");
                let mut fd = File::create(&temp_path)?;
                fd.write_all(&response_bytes)?;
                target_path = temp_path
                    .to_str()
                    .ok_or(anyhow!("fail to get path"))?
                    .to_string();
                info!(
                    "Save Http/s file to  <{:#?}> => dst: <{:#?}>",
                    &path_or_url, &target_path
                );
                _temp_dir_guard = Some(temp_dir);
            } else {
                target_path = path_or_url.to_string();
                _temp_dir_guard = None;
            }
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
                    Ok(())
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
            if status {
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
            if status {
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
            self.shell(build_uninstall_command(package_name)).await
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

            Some(common::populate_app_info(package_name, &app_info_output))
        }
        pub async fn if_screen_on(&mut self) -> AdbResult<bool> {
            let resp = self.shell(["dumpsys", "power"]).await?;
            Ok(resp.contains("mHoldingDisplaySuspendBlocker=true"))
        }

        pub async fn remove(&mut self, path: &str) -> AdbResult<String> {
            self.shell_trim(["rm", path]).await
        }

        pub async fn get_sdk_version(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::SdkVersion).await
        }

        pub async fn get_android_version(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::AndroidVersion).await
        }

        pub async fn get_device_model(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::DeviceModel).await
        }

        pub async fn get_device_brand(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::DeviceBrand).await
        }
        pub async fn get_device_manufacturer(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::DeviceManufacturer)
                .await
        }
        pub async fn get_device_product(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::DeviceProduct).await
        }

        pub async fn get_device_abi(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::DeviceAbi).await
        }

        pub async fn get_device_gpu(&mut self) -> AdbResult<String> {
            let resp = self.shell(["dumpsys", "SurfaceFlinger"]).await;
            if let Ok(data) = resp {
                for x in data.lines() {
                    if x.starts_with("GLES:") {
                        return Ok(x.to_string());
                    }
                }
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
    use crate::client::common::{
        self, DeviceTextCommand, build_forward_command, build_reverse_command,
        build_uninstall_command, command_output_to_result, extract_forward_item_from_output,
        extract_ip_from_output, extract_port_from_tcp_spec,
    };
    use crate::client::AdbDevice;
    use crate::errors::{AdbError, AdbResult};
    use crate::protocols::AdbProtocol;
    use crate::sync_protocol::{self, RecvFrame};
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

        fn run_text_command(&mut self, command: DeviceTextCommand) -> AdbResult<String> {
            match command.transport_command() {
                Some(cmd) => self.get_with_command(cmd),
                None => {
                    let out = self.shell(command.shell_args().unwrap())?;
                    if command.trim_output() {
                        Ok(out.trim().to_string())
                    } else {
                        Ok(out)
                    }
                }
            }
        }

        pub fn get_state(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::State)
        }

        pub fn get_serialno(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::SerialNo)
        }

        pub fn get_devpath(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::DevPath)
        }

        pub fn get_features(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::Features)
        }

        /// 执行通过ADB shell命令流，并返回一个AdbConnection的实例。
        ///
        /// # 参数
        /// - `command`: 一个包含多个命令参数的字符串切片数组，每个元素都是一个命令参数。
        ///
        /// # 返回值
        /// - `AdbResult<AdbConnection>`: 如果命令成功执行，则返回一个AdbConnection的实例；否则返回错误信息。
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
            let full_cmd = build_forward_command(local, remote, norebind);
            self.open_transport(Some(&full_cmd))?;
            Ok(())
        }

        pub fn forward_list(&mut self) -> AdbResult<Vec<ForwardItem>> {
            let mut connection = self.open_transport(Some("list-forward"))?;
            let content = connection.read_response()?;
            Ok(extract_forward_item_from_output(&content))
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
            let full_cmd = build_reverse_command(remote, local, norebind);
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
                let output = cmd.output()?;
                return command_output_to_result(command, output);
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
            self.adb_output(&["push", local, remote])?;
            info!("push {} to {} success", local, remote);
            Ok(())
        }
        pub fn pull(&mut self, src: &str, dest: &PathBuf) -> AdbResult<usize> {
            let mut size = 0;
            let mut file = File::create(dest)?;
            for content in self.iter_content(src)? {
                match content {
                    Ok(content) => {
                        file.write_all(&content)?;
                        size += content.len();
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(size)
        }

        pub fn iter_directory(&mut self, path: &str) -> AdbResult<impl Iterator<Item = FileInfo>> {
            let mut conn = self.prepare_sync(path, "LIST")?;
            Ok(std::iter::from_fn(move || {
                let data = conn.read_string(4).ok()?;
                if data.eq("DONE") {
                    None
                } else {
                    let current_data = conn.recv(16).ok()?;
                    let name_length_bytes = &current_data[12..=15];
                    let name_length = u32::from_le_bytes(name_length_bytes.try_into().unwrap());
                    let path = conn.read_string(name_length as usize).ok()?;
                    Some(parse_file_info(current_data, path).ok()?)
                }
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
            let chunks = self
                .iter_content(path)?
                .collect::<AdbResult<Vec<Vec<u8>>>>()?;
            let bytes = chunks.concat();
            Ok(String::from_utf8_lossy(&bytes).to_string())
        }

        pub fn prepare_sync(&mut self, path: &str, command: &str) -> AdbResult<TcpStream> {
            info!("Start Sync Path {:#?} With Command {:#?}", path, command);
            let mut conn = self.open_transport(None)?;
            conn.send_cmd_then_check_okay("sync:")
                .context("Start Sync Error")?;
            let command_id: [u8; 4] = command
                .as_bytes()
                .try_into()
                .map_err(|_| AdbError::protocol_error("sync command must be 4 bytes"))?;
            let total_byte = sync_protocol::encode_path_command(&command_id, path);
            conn.send_all(&total_byte)?;
            Ok(conn)
        }

        pub fn iter_content(
            &mut self,
            path: &str,
        ) -> AdbResult<impl Iterator<Item = AdbResult<Vec<u8>>>> {
            if let Ok(mut connection) = self.prepare_sync(path, "RECV") {
                let mut done = false;
                return Ok(std::iter::from_fn(move || {
                    if done {
                        return None;
                    }
                    match sync_protocol::read_recv_frame(&mut connection) {
                        Ok(RecvFrame::Data(data)) => Some(Ok(data)),
                        Ok(RecvFrame::Done) => {
                            done = true;
                            None
                        }
                        Ok(RecvFrame::Fail(message)) => {
                            done = true;
                            error!("Sync Error With Error Message >>> {:#?}", message);
                            Some(Err(AdbError::protocol_error(message)))
                        }
                        Err(e) => {
                            done = true;
                            Some(Err(e))
                        }
                    }
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
            if status {
                self.keyevent("224")
            } else {
                self.keyevent("223")
            }
        }

        pub fn install(&mut self, path_or_url: &str) -> AdbResult<()> {
            let _temp_dir_guard;
            let target_path;
            if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
                let mut resp = reqwest::blocking::get(path_or_url).context("Http Request Error")?;
                let mut buffer = Vec::new();
                resp.read_to_end(&mut buffer)?;
                let temp_dir = tempfile::tempdir()?;
                let temp_path = temp_dir.path().join("tmp001.apk");
                let mut fd = File::create(&temp_path)?;
                fd.write_all(&buffer)?;
                target_path = temp_path
                    .to_str()
                    .ok_or(AdbError::file_operation_failed(
                        "getTempDir",
                        "fail to get path",
                    ))?
                    .to_string();
                info!(
                    "Save Http/s file to  <{:#?}> => dst: <{:#?}>",
                    &path_or_url, &target_path
                );
                _temp_dir_guard = Some(temp_dir);
            } else {
                target_path = path_or_url.to_string();
                _temp_dir_guard = None;
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
            if status {
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
            if status {
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
            self.shell(build_uninstall_command(package_name))
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

            Some(common::populate_app_info(package_name, &app_info_output))
        }

        pub fn if_screen_on(&mut self) -> AdbResult<bool> {
            let resp = self.shell(["dumpsys", "power"])?;
            Ok(resp.contains("mHoldingDisplaySuspendBlocker=true"))
        }

        pub fn remove(&mut self, path: &str) -> AdbResult<String> {
            self.shell_trim(["rm", path])
        }

        pub fn get_sdk_version(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::SdkVersion)
        }

        pub fn get_android_version(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::AndroidVersion)
        }

        pub fn get_device_model(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::DeviceModel)
        }

        pub fn get_device_brand(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::DeviceBrand)
        }
        pub fn get_device_manufacturer(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::DeviceManufacturer)
        }
        pub fn get_device_product(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::DeviceProduct)
        }

        pub fn get_device_abi(&mut self) -> AdbResult<String> {
            self.run_text_command(DeviceTextCommand::DeviceAbi)
        }

        pub fn get_device_gpu(&mut self) -> AdbResult<String> {
            let resp = self.shell(["dumpsys", "SurfaceFlinger"]);
            if let Ok(data) = resp {
                for x in data.lines() {
                    if x.starts_with("GLES:") {
                        return Ok(x.to_string());
                    }
                }
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
