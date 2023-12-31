use crate::client::device::BaseDevice;
use crate::connections::adb_connection::AdbConnection;
use crate::connections::base_client::BaseClient;
use anyhow::{anyhow, Result};
use log::info;
use std::collections::HashMap;
use std::fmt::Display;
use std::io::BufRead;



pub struct AdbClient {
    pub c: BaseClient,
}

#[derive(Debug)]
pub struct AdbDeviceInfo {
    pub serial: String,
    pub state: String,
    pub properties: HashMap<String, String>,
}

impl AdbDeviceInfo {
    fn new(serial: String, state: String) -> AdbDeviceInfo {
        AdbDeviceInfo {
            serial,
            state,
            properties: HashMap::new(),
        }
    }
}

impl Default for AdbClient {
    fn default() -> Self {
        Self {
            c: BaseClient::default(),
        }
    }
}

impl AdbClient {
    pub fn new<T: ToString>(host: T, port: u16, socket_timeout: u32) -> Result<AdbClient> {
        let adb_client = AdbClient {
            c: BaseClient::new(host.to_string(), port, socket_timeout),
        };
        Ok(adb_client)
    }

    fn _connect(&mut self, timeout: Option<u32>) -> Result<AdbConnection> {
        if let Ok(mut connection) = self.c.connect() {
            return if timeout.as_ref().is_none() {
                Ok(connection)
            } else {
                connection.set_timeout(timeout.unwrap())?;
                Ok(connection)
            };
        } else {
            Err(anyhow!("Connect Failed"))
        }
    }

    pub fn list_devices(&mut self) -> Result<Vec<AdbDeviceInfo>> {
        let mut conn = self._connect(None)?;
        let mut devices = vec![];
        let cmd = "host:devices";
        conn.send_command(cmd)?;
        conn.check_okay()?;
        let resp = conn.read_string_block()?;
        for line in resp.lines() {
            let parts: Vec<&str> = line.split("\t").collect();
            if !parts.is_empty() {
                devices.push(AdbDeviceInfo::new(
                    parts[0].to_string(),
                    parts[1].to_string(),
                ));
            }
        }
        Ok(devices)
    }

    pub fn iter_devices(&mut self) -> Result<impl Iterator<Item = AdbDeviceInfo>> {
        if let Ok(devices) = self.list_devices(){
            return Ok(devices.into_iter())
        }
        Err(anyhow!("Failed To Get Device List"))
    }

    fn get_the_only_one_device(&mut self) -> Result<BaseDevice>{
        let mut devices = self.list_devices()?;
        if devices.len()!= 1 {
            return Err(anyhow!("There are {} devices, Please Pass Serial Or TransportId", devices.len()));
        }
        Ok(BaseDevice::new(
            self.c.clone(),
            Some((&devices[0].serial).to_string()),
            None,
        ))
    }

    pub fn get_device_by_serial(&mut self, serial: Option<&str>) -> Result<BaseDevice> {
        if serial.is_some(){
            return Ok(BaseDevice::new(
                self.c.clone(),
                Option::from(serial.unwrap().to_string()),
                None,
            ));
        }
        self.get_the_only_one_device()

    }

    pub fn get_device_by_transport_id(&mut self, transport_id: Option<u8>) -> Result<BaseDevice>{
        if transport_id.is_some(){
            return Ok(
                BaseDevice::new(
                self.c.clone(),
                None,
                transport_id
            ))
        }
        self.get_the_only_one_device()
    }


    fn execute_function_with_connection<F: Fn(AdbConnection) -> Result<String>>(
        &mut self,
        f: F,
        timeout: Option<u32>
    ) -> Result<String> {
        if let Ok(mut connection) = self._connect(timeout) {
            if let Ok(resp) = f(connection) {
                return Ok(resp);
            }
        }
        Err(anyhow!("Failed to connect to Adb Server"))
    }

    pub fn server_version(&mut self) -> Result<String> {
        self.execute_function_with_connection(|mut conn| {
            let cmd = "host:version";
            conn.send_command(cmd)?;
            conn.check_okay()?;
            let version_str: String = conn.read_string_block()?;
            let current_version = usize::from_str_radix(&version_str, 16)?;
            Ok(current_version.to_string())
        }, None)
            .map_err(|err| anyhow!("Get Adb Server Version Error: {}", err))
    }

    pub fn server_kill(&mut self,) -> Result<String> {
        self.execute_function_with_connection(|mut conn| {
            let cmd = "host:kill";
            conn.send_command(cmd)?;
            conn.check_okay()?;
            Ok("Success".to_string())
        },
        None)
    }

    pub fn connect(&mut self, serial: &str, timeout: Option<u32>) -> Result<String> {
        let cmd = format!("host:connect:{}", serial);
        self.execute_function_with_connection(|mut conn| {
            conn.send_command(&cmd)?;
            conn.check_okay()?;
            let resp = conn.read_string_block()?;
            Ok(resp)
        },timeout)
    }

    pub fn disconnect(&mut self, serial: &str,) -> Result<String> {
        if serial.is_empty() {
            return Err(anyhow!("addr is empty"));
        }
        self.execute_function_with_connection(|mut conn| {
            conn.send_command(&format!("host:disconnect:{}", serial))?;
            conn.check_okay()?;
            if let Ok(resp) = conn.read_string_block(){
                info!(
            "Disconnect To <{:#?}> Succeed With Response {:#?}",
            serial, &resp
        );
                return Ok(resp)
            }
            Err(anyhow!("Failed To Disconnect Device"))
        },None)
    }

}
