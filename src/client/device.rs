use crate::connections::adb_connection::AdbConnection;
use crate::connections::base_client::BaseClient;
use anyhow::{anyhow, Result};
use chrono::DateTime;
use image::io::Reader as ImageReader;
use image::RgbImage;
use log::info;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::Command;
use std::str::FromStr;
use std::{fmt, fs, time, vec};
use std::path::PathBuf;
use crate::beans::app_info::AppInfo;
use crate::beans::file_info::{FileInfo, parse_file_info};
use crate::beans::forward_item::ForwardIterm;
use crate::beans::net_info::NetworkType;
use crate::utils::{adb_path, get_free_port};


#[derive(Clone)]
pub struct BaseDevice {
    pub client: BaseClient,
    pub serial: Option<String>,
    pub transport_id: Option<u8>,
    pub properties: HashMap<String, String>,
}

impl fmt::Debug for BaseDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BaseDevice")
            .field("serial", &self.serial)
            .field("transport_id", &self.transport_id)
            .field("properties", &self.properties)
            .finish()
    }
}


pub struct DeviceInfo {
    pub(crate) serialno: String,
    pub(crate) devpath: String,
    pub(crate) state: String,
}





fn humanize(size: f64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut size = size;
    let mut unit_index = 0;
    while size >= 1024.0 && unit_index < units.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    format!("{:.1}{}", size, units[unit_index])
}



impl DeviceInfo {
    fn new(serialno: String, devpath: String, state: String) -> DeviceInfo {
        DeviceInfo {
            serialno,
            devpath,
            state,
        }
    }
}

impl Display for BaseDevice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BaseDevice({})",
            self.serial.as_ref().unwrap_or(&"None".to_string())
        )
    }
}

impl BaseDevice {

    pub fn new(client: BaseClient, serial: Option<String>, transport_id: Option<u8>) -> BaseDevice {
        if serial.is_none() && transport_id.is_none() {
            panic!("serial or transport_id must be set");
        }

        BaseDevice {
            client,
            serial,
            transport_id,
            properties: HashMap::new(),
        }
    }

    fn open_transport_with_command(&mut self, command: Option<&str>,  connection: &mut AdbConnection) -> Result<()>{
        match command {
            Some(cmd) => {
                if self.transport_id.is_some() {
                    let send_cmd =
                        format!("host-transport-id:{}:{}", self.transport_id.unwrap(), cmd);
                    connection.send_command(send_cmd.as_str())?;
                } else if self.serial.as_ref().is_some()
                    & !&self.serial.as_ref().is_some_and(|x| x.is_empty())
                {
                    let send_cmd =
                        format!("host-serial:{}:{}", self.serial.clone().unwrap(), cmd);
                    connection.send_command(send_cmd.as_str())?;
                } else {
                    panic!("serial or transport_id must be set");
                }
                connection.check_okay()?;
            }
            _ => {}
        }
        Ok(())
    }


    fn get_connection_with_timeout(&mut self, timeout: Option<u32>) -> Result<AdbConnection>{
        let mut c = self.client.connect()?;
        if timeout.is_some() {
            c.set_timeout(timeout.unwrap())?
        }
        Ok(c)
    }


    fn open_transport_without_command(&mut self, connection: &mut AdbConnection) -> Result<()> {
        if self.transport_id.is_some() {
            let send_cmd = format!("host-transport-id:{}", self.transport_id.clone().unwrap());
            connection.send_command(send_cmd.as_str())?;
        } else if self.serial.as_ref().is_some()
            & !self.serial.as_ref().is_some_and(|x| x.is_empty())
        {
            let send_cmd = format!("host:transport:{}", self.serial.clone().unwrap());
            connection.send_command(send_cmd.as_str())?;
        } else {
            panic!("serial or transport_id must be set");
        }
        connection.check_okay()?;
        Ok(())
    }

    pub fn open_transport(
        &mut self,
        command: Option<&str>,
        timeout: Option<u32>,
    ) -> Result<AdbConnection> {
        let mut c = self.get_connection_with_timeout(timeout)?;
        let x= if command.is_some() & !command.is_some_and(|x| x.is_empty()) {
            self.open_transport_with_command(
                command, &mut c
            )
        } else {
            self.open_transport_without_command(&mut c)
        };
        if x.is_ok(){
            Ok(c)
        }else {
            Err(anyhow!("Failed to open transport"))
        }
    }

    pub fn get_with_command(&mut self, command: &str) -> Result<String> {
        let mut c = self.open_transport(Some(command), None)?;
        let result = c.read_string_block()?;
        Ok(result)
    }

    pub fn get_state(&mut self) -> Result<String> {
        self.get_with_command("get-state")
    }

    pub fn get_serialno(&mut self) -> Result<String> {
        self.get_with_command("get-serialno")
    }

    pub fn get_devpath(&mut self) -> Result<String> {
        self.get_with_command("get-devpath")
    }

    pub fn get_features(&mut self) -> Result<String> {
        self.get_with_command("get-features")
    }

    pub fn get_info(&mut self) -> Result<DeviceInfo> {
        Ok(DeviceInfo::new(
            self.get_serialno()?,
            self.get_devpath()?,
            self.get_state()?,
        ))
    }

    fn list2cmdline(args: &[&str]) -> String {
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

    pub fn shell_stream(&mut self, command: &[&str]) -> Result<AdbConnection> {
        let mut conn = self.open_transport(None, None)?;
        let cmd = Self::list2cmdline(command);
        let send_cmd = format!("shell:{}", cmd);
        conn.send_command(send_cmd.as_str())?;
        conn.check_okay()?;
        Ok(conn)
    }

    pub fn shell(&mut self, command: &[&str]) -> Result<String> {
        let mut s = self.shell_stream(command)?;
        let output = s.read_until_close()?;
        Ok(output)
    }

    pub fn shell_trim(&mut self, command: &[&str]) -> Result<String> {
        let mut s = self.shell_stream(command)?;
        let output = s.read_until_close()?;
        Ok(output.trim().to_string())
    }

    pub fn forward(&mut self, local: &str, remote: &str, norebind: bool) -> Result<()> {
        let mut args = vec!["forward"];
        if norebind {
            args.push("norebind");
        }
        let forward_str = format!("{};{}", local, remote);
        args.push(&forward_str);
        let full_cmd = args.join(":");
        if let Ok(resp) = self.open_transport(Some(&full_cmd), None){
            return Ok(())
        }
        Err(anyhow!("Failed To Forward Port"))
    }

    pub fn forward_remote_port(&mut self, remote: u16) -> Result<u16> {
        let remote = format!("tcp:{}", remote);
        for x in self.forward_list()? {
            if x.serial.eq(self.serial.as_ref().unwrap().as_str())
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

    pub fn forward_list(&mut self) -> Result<Vec<ForwardIterm>> {
        let mut connection = self.open_transport(Some("list-forward"), None)?;
        let content = connection.read_string_block()?;
        let mut forward_iterms = vec![];
        for x in content.lines() {
            let mut current_parts:Vec<&str>= x.split(" ").collect();
            if current_parts.len() == 3{
                let (serial, local, remote) = (current_parts[0], current_parts[1], current_parts[2]);
                forward_iterms.push(ForwardIterm::new(serial, local, remote))
            }
        }
        Ok(forward_iterms)
    }

    pub fn reverse(&mut self, remote: &str, local: &str, norebind: bool) -> Result<()> {
        let mut args = vec!["forward"];
        if norebind {
            args.push("norebind");
        }
        args.push(local);
        args.push(";");
        args.push(remote);
        let full_cmd = args.join(":");
        self.open_transport(Some(&full_cmd), None)?;
        Ok(())
    }

    pub fn adb_output(&mut self, command: &[&str]) -> Result<String> {
        let adb_ = adb_path()?;
        if adb_.exists() {
            let mut cmd = Command::new(adb_.to_str().unwrap());
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

    pub fn push(&mut self, local: &str, remote: &str) -> Result<()> {
        if self.adb_output(&["push", local, remote]).is_ok() {
            info!("push {} to {} success", local, remote);
            return Ok(());
        }
        Err(anyhow!("push error"))
    }

    pub fn create_connection<T: Display>(
        &mut self,
        network_type: NetworkType,
        address: T,
    ) -> Result<AdbConnection> {
        let mut connection = self.open_transport(None, None)?;
        match network_type {
            NetworkType::LocalAbstrcat | NetworkType::Unix => {
                let s = format!("{}{}", "localabstract:", address);
                connection.send_command(&s)?;
                connection.check_okay()?;
            }
            _ => {
                let s = format!("{}{}", network_type.to_string(), address);
                connection.send_command(&s)?;
                connection.check_okay()?;
            }
        }
        Ok(connection)
    }


    pub fn tcpip(&mut self, port: u16) -> Result<String> {
        let mut connection = self.open_transport(None, None)?;
        let s = format!("tcpip:{}", port);
        connection.send_command(&s)?;
        connection.check_okay()?;
        let resp = connection.read_until_close()?;
        Ok(resp)
    }

    pub fn screenshot(&mut self) -> Result<RgbImage> {
        let src = "/sdcard/screen.png";
        self.shell(&["screencap", "-p", src])?;
        let tmpdir = tempfile::tempdir().expect("Failed to create temporary directory");
        let target_path = tmpdir.path().join("tmp001.png");
        info!("Pull Image To {:#?}", &target_path);
        self.pull(src, &target_path)?;
        self.shell(&["rm", src])?;

        let image = ImageReader::open(&target_path)?
            .decode()?;
        fs::remove_file(target_path).expect("Failed to remove file");
        Ok(image.into_rgb8())
    }

    pub fn switch_screen(&mut self, status: bool) -> Result<String> {
        if status == true {
            self.keyevent("224")
        } else {
            self.keyevent("223")
        }
    }

    pub fn switch_airplane_mode(&mut self, status: bool) -> Result<String> {
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

    pub fn keyevent(&mut self, keycode: &str) -> Result<String> {
        self.shell(&["input", "keyevent", keycode])
    }

    pub fn switch_wifi(&mut self, status: bool) -> Result<String> {
        let mut args = vec!["svc", "wifi"];
        if status == true {
            args.push("enable");
        } else {
            args.push("disable");
        };
        self.shell(&args)
    }

    pub fn click(&mut self, x: i32, y: i32) -> Result<String> {
        self.shell(&["input", "tap", &x.to_string(), &y.to_string()])
    }

    pub fn swipe(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, duration: i32) -> Result<String> {
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

    pub fn send_keys(&mut self, keys: &str) -> Result<String> {
        self.shell(&["input", "text", keys])
    }

    pub fn wlan_ip(&mut self) -> Result<String> {
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

    pub fn uninstall(&mut self, package_name: &str) -> Result<String> {
        self.shell(&["pm", "uninstall", package_name])
    }


    pub fn get_prop(&mut self, property_key: &str) -> Result<String> {
        let property_key_str = property_key.to_string();
        if self.properties.get(&property_key_str).is_some(){
            return Ok(self.properties[property_key].clone());
        }
        if let Ok(property_value) = self.shell_trim(&["getprop", &property_key]) {
            self.properties.insert(property_key_str, property_value.clone());
            return Ok(property_value)
        }
        Err(anyhow!("Get Prop Failed"))
    }

    pub fn app_start(&mut self, package_name: &str) -> Result<String> {
        self.shell(&["am", "start", "-n", package_name])
    }

    pub fn app_stop(&mut self, package_name: &str) -> Result<String> {
        self.shell(&["am", "force-stop", package_name])
    }

    pub fn app_clear_data(&mut self, package_name: &str) -> Result<String> {
        self.shell(&["pm", "clear", package_name])
    }

    pub fn install(&mut self, path_or_url: &str) -> Result<(), anyhow::Error> {
        let target_path =
            if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
                let mut resp = reqwest::blocking::get(path_or_url)?;
                let mut buffer = Vec::new();
                resp.read_to_end(&mut buffer)?;
                let temp_dir = tempfile::tempdir()?.path().join("tmp001.apk");
                let mut fd = File::create(&temp_dir)?;
                fd.write_all(&buffer)?;
                let target_path = temp_dir
                    .to_str()
                    .ok_or(anyhow!("fail to get path"))?;
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

    pub fn install_remote(&mut self, path: &str, clean: bool) -> Result<String> {
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

    pub fn if_screen_on(&mut self) -> Result<bool> {
        let resp = self.shell(&["dumpsys", "power"])?;
        Ok(resp.contains("mHoldingDisplaySuspendBlocker=true"))
    }

    pub fn remove(&mut self, path: &str) -> Result<String> {
        self.shell(&["rm", path])
    }

    pub fn get_sdk_version(&mut self) -> Result<String> {
        self.shell(&["getprop", "ro.build.version.sdk"])
    }

    pub fn get_android_version(&mut self) -> Result<String> {
        self.shell(&["getprop", "ro.build.version.release"])
    }

    pub fn get_device_model(&mut self) -> Result<String> {
        self.shell(&["getprop", "ro.product.model"])
    }

    pub fn get_device_brand(&mut self) -> Result<String> {
        self.shell(&["getprop", "ro.product.brand"])
    }
    pub fn get_device_manufacturer(&mut self) -> Result<String> {
        self.shell(&["getprop", "ro.product.manufacturer"])
    }
    pub fn get_device_product(&mut self) -> Result<String> {
        self.shell(&["getprop", "ro.product.product"])
    }

    pub fn get_device_abi(&mut self) -> Result<String> {
        self.shell(&["getprop", "ro.product.cpu.abi"])
    }

    pub fn get_device_gpu(&mut self) -> Result<String> {
        let resp = self.shell(&["dumpsys", "SurfaceFlinger",]);
        match resp {
            Ok(data) => {
                for x in data.split("\n") {
                    if x.starts_with("GLES:"){
                        return Ok(x.to_string())
                    }
                }
            }
            _ => {}
        }
        Err(anyhow!("fail to get gpu"))
    }

    pub fn logcat(&mut self, flush_exist: bool) -> Result<impl Iterator<Item = String>>{
        if (flush_exist){
            self.shell(&["logcat", "-c"])?;
        }
        return if let Ok(mut conn) = self.shell_stream(&["logcat"]) {
            Ok(
                std::iter::from_fn(
                    move || {
                        let mut bufreader = BufReader::new(&conn.conn);

                        loop {
                            let mut string = String::new();
                            let data = bufreader.read_line(&mut string);
                            return Some(string)
                        }
                    }
                )
            )
        } else {
            Err(anyhow!("fail to get logcat"))
        }
    }


    pub fn prepare_sync(&mut self, path: &str, command: &str) -> Result<AdbConnection> {
        let serial = self.serial.clone().unwrap();
        if let Ok(mut conn) = self.client.connect() {
            let cmd = vec!["host", "transport", &serial];
            let send_cmd = cmd.join(":");
            conn.send_command(&send_cmd)?;
            conn.check_okay()?;
            conn.send_command("sync:")?;
            conn.check_okay()?;
            let path_len = path.as_bytes().len() as u32;
            let mut total_byte = vec![];
            total_byte.extend_from_slice(command.as_bytes());
            total_byte.extend_from_slice(&path_len.to_le_bytes());
            total_byte.extend_from_slice(path.as_bytes());
            conn.send(&total_byte)?;
            return Ok(conn)
        }
        Err(anyhow!("fail to connect"))
    }

    pub fn exists(&mut self, path: &str) -> Result<bool> {
        let file_info = self.stat(path)?;
        if file_info.mtime != 0 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn stat(&mut self, path: &str) -> Result<FileInfo> {
        let mut conn = self.prepare_sync(path, "STAT")?;
        let data = conn.read_string(4)?;
        if data.eq("STAT") {
            let current_data = conn.read(12)?;
            return Ok(parse_file_info(current_data, path)?);
        };
        Err(anyhow!("stat error"))
    }

    pub fn iter_directory(&mut self, path: & str) -> Result<impl Iterator<Item = FileInfo>> {
        let mut conn = self.prepare_sync(path, "LIST")?;
        Ok(std::iter::from_fn(move || {
            let data = conn.read_string(4).ok()?;
            return if data.eq("DONE") {
                None
            } else {
                let mut current_data = conn.read(16).ok()?;
                let name_length_bytes = &current_data[12..=15];
                let name_length = u32::from_le_bytes(name_length_bytes.try_into().unwrap());
                let path = conn.read_string(name_length as usize).ok()?;
                Some(parse_file_info(current_data, path).ok()?)
            }
        }))
    }

    pub fn list(&mut self, path: & str) -> Vec<FileInfo> {
        self.iter_directory(path).unwrap().collect()
    }

    pub fn iter_content(&mut self, path: & str) -> Result<impl Iterator<Item = Result<String>>> {
        if let Ok(mut connection) = self.prepare_sync(path, "RECV") {
            let mut done = false;
            return Ok(std::iter::from_fn(move || {
                if done {
                    return None;
                }
                return match connection.read_string(4) {
                    Err(_) => None,
                    Ok(data) => match data.as_str() {
                        "FAIL" => match connection.read(4) {
                            Err(_) => None,
                            Ok(data) => {
                                let str_size = u32::from_le_bytes(data.try_into().ok()?) as usize;
                                match connection.read(str_size) {
                                    Err(_) => None,
                                    Ok(data) => {
                                        let content = String::from_utf8_lossy(&data).to_string();
                                        Some(Ok(content))
                                    }
                                }
                            }
                        },
                        "DONE" => {
                            done = true;
                            None
                        }
                        "DATA" => match connection.read(4) {
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

    pub fn read_text(&mut self, path: & str) -> Result<String> {
        let data = self
            .iter_content(path)?
            .map(|x| x.unwrap_or_else(|_| "".to_string()))
            .collect::<Vec<_>>();
        Ok(data.join(""))
    }

    pub fn pull(&mut self, src: & str, dest: &PathBuf) -> Result<usize> {
        let mut size = 0;
        let mut file = match File::open(dest) {
            Ok(mut file) => {
                file
            }
            Err(_) => {
                File::create(dest)?
            }
        };
        self.iter_content(src)
            .unwrap()
            .for_each(|content| match content {
                Ok(content) => {
                    file.write_all(content.as_bytes()).unwrap();
                    size += content.len();
                }
                Err(_) => {}
            });
        Ok(size)
    }
}
