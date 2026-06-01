use crate::beans::{AppInfo, ForwardItem};
use crate::errors::{AdbError, AdbResult};
use chrono::{DateTime, NaiveDateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use std::process::{ExitStatus, Output};

static IP_REGEXES: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"inet\s+addr:([\d.]+)").unwrap(),
        Regex::new(r"inet\s+([\d.]+)/\d+").unwrap(),
        Regex::new(r"inet\s+([\d.]+)\s+netmask").unwrap(),
    ]
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceListEntry {
    pub serial: String,
    pub state: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceTextCommand {
    State,
    SerialNo,
    DevPath,
    Features,
    SdkVersion,
    AndroidVersion,
    DeviceModel,
    DeviceBrand,
    DeviceManufacturer,
    DeviceProduct,
    DeviceAbi,
}

impl DeviceTextCommand {
    pub fn transport_command(&self) -> Option<&'static str> {
        match self {
            DeviceTextCommand::State => Some("get-state"),
            DeviceTextCommand::SerialNo => Some("get-serialno"),
            DeviceTextCommand::DevPath => Some("get-devpath"),
            DeviceTextCommand::Features => Some("get-features"),
            _ => None,
        }
    }

    pub fn shell_args(&self) -> Option<&'static [&'static str]> {
        match self {
            DeviceTextCommand::SdkVersion => Some(&["getprop", "ro.build.version.sdk"]),
            DeviceTextCommand::AndroidVersion => Some(&["getprop", "ro.build.version.release"]),
            DeviceTextCommand::DeviceModel => Some(&["getprop", "ro.product.model"]),
            DeviceTextCommand::DeviceBrand => Some(&["getprop", "ro.product.brand"]),
            DeviceTextCommand::DeviceManufacturer => {
                Some(&["getprop", "ro.product.manufacturer"])
            }
            DeviceTextCommand::DeviceProduct => Some(&["getprop", "ro.product.product"]),
            DeviceTextCommand::DeviceAbi => Some(&["getprop", "ro.product.cpu.abi"]),
            _ => None,
        }
    }

    pub fn trim_output(&self) -> bool {
        self.shell_args().is_some()
    }
}

pub fn build_forward_command(local: &str, remote: &str, norebind: bool) -> String {
    if norebind {
        format!("forward:norebind:{};{}", local, remote)
    } else {
        format!("forward:{};{}", local, remote)
    }
}

pub fn build_reverse_command(remote: &str, local: &str, norebind: bool) -> String {
    if norebind {
        format!("reverse:norebind:{};{}", remote, local)
    } else {
        format!("reverse:{};{}", remote, local)
    }
}

pub fn build_uninstall_command(package_name: &str) -> [&str; 3] {
    ["pm", "uninstall", package_name]
}

pub fn build_transport_prefix(
    serial: Option<&str>,
    transport_id: Option<u8>,
    command: Option<&str>,
) -> AdbResult<String> {
    match (serial, transport_id, command) {
        (_, Some(id), Some(command)) => Ok(format!("host-transport-id:{}:{}", id, command)),
        (Some(serial), None, Some(command)) => Ok(format!("host-serial:{}:{}", serial, command)),
        (_, Some(id), None) => Ok(format!("host-transport-id:{}", id)),
        (Some(serial), None, None) => Ok(format!("host:transport:{}", serial)),
        (None, None, _) => Err(AdbError::protocol_error(
            "TransportID and Serial Can Not Been None At Same Time",
        )),
    }
}

pub fn parse_device_list(lines: &str) -> Vec<DeviceListEntry> {
    lines
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let serial = parts.next()?.to_string();
            let state = parts.next().map(str::to_string);
            Some(DeviceListEntry { serial, state })
        })
        .collect()
}

pub fn adb_output_to_result(status: ExitStatus, stdout: &str, stderr: &str) -> AdbResult<String> {
    if status.success() {
        Ok(stdout.to_string())
    } else {
        let reason = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        Err(AdbError::command_failed("adb", reason))
    }
}

pub fn command_output_to_result(command: &[&str], output: Output) -> AdbResult<String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        Ok(stdout.to_string())
    } else {
        let reason = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        Err(AdbError::command_failed(command.join(" "), reason))
    }
}

pub fn extract_forward_item_from_output(output: &str) -> Vec<ForwardItem> {
    output
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
        .collect()
}

pub fn is_valid_ipv4(ip: &str) -> bool {
    let mut parts = ip.split('.');
    let valid_parts = parts
        .by_ref()
        .take(4)
        .all(|part| !part.is_empty() && part.parse::<u8>().is_ok());
    valid_parts && parts.next().is_none() && ip.split('.').count() == 4
}

pub fn extract_ip_from_output(output: &str) -> Option<String> {
    for regex in IP_REGEXES.iter() {
        if let Some(captures) = regex.captures(output) {
            if let Some(ip_match) = captures.get(1) {
                let ip = ip_match.as_str();
                if is_valid_ipv4(ip) {
                    return Some(ip.to_string());
                }
            }
        }
    }
    None
}

pub fn extract_port_from_tcp_spec(tcp_spec: &str) -> Option<u16> {
    tcp_spec.strip_prefix("tcp:")?.parse().ok()
}

pub fn escape_shell_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }

    if !arg.chars().any(|c| " \"'\\$`(){}[]|&;<>?*~".contains(c)) {
        return arg.to_string();
    }

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

pub fn extract_app_version_info(output: &str, app_info: &mut AppInfo) {
    if let Ok(version_name_regex) = Regex::new(r"versionName=(\S+)") {
        if let Some(cap) = version_name_regex.captures(output) {
            if let Some(version_name) = cap.get(1) {
                app_info.version_name = Some(version_name.as_str().to_string());
            }
        }
    }

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

pub fn extract_app_signature(output: &str, app_info: &mut AppInfo) {
    if let Ok(signature_regex) = Regex::new(r"PackageSignatures\{[^}]*\[([^]]+)]") {
        if let Some(cap) = signature_regex.captures(output) {
            if let Some(signature) = cap.get(1) {
                app_info.signature = Some(signature.as_str().to_string());
            }
        }
    }
}

pub fn extract_app_flags(output: &str, app_info: &mut AppInfo) {
    if let Ok(flags_regex) = Regex::new(r"pkgFlags=\[\s*([^]]+)\s*]") {
        if let Some(cap) = flags_regex.captures(output) {
            if let Some(flags_str) = cap.get(1) {
                app_info.flags = flags_str
                    .as_str()
                    .split_whitespace()
                    .map(str::to_string)
                    .collect();
            }
        }
    }
}

pub fn extract_app_timestamps(output: &str, app_info: &mut AppInfo) {
    if let Ok(first_install_regex) = Regex::new(r"firstInstallTime=([\d-]+\s+[:\d]+)") {
        if let Some(cap) = first_install_regex.captures(output) {
            if let Some(time_str) = cap.get(1) {
                app_info.first_install_time = parse_android_datetime(time_str.as_str());
            }
        }
    }

    if let Ok(last_update_regex) = Regex::new(r"lastUpdateTime=([\d-]+\s+[:\d]+)") {
        if let Some(cap) = last_update_regex.captures(output) {
            if let Some(time_str) = cap.get(1) {
                app_info.last_update_time = parse_android_datetime(time_str.as_str());
            }
        }
    }
}

pub fn populate_app_info(package_name: &str, output: &str) -> AppInfo {
    let mut app_info = AppInfo::new(package_name);
    extract_app_version_info(output, &mut app_info);
    extract_app_signature(output, &mut app_info);
    extract_app_flags(output, &mut app_info);
    extract_app_timestamps(output, &mut app_info);
    app_info
}

fn parse_android_datetime(input: &str) -> Option<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}
