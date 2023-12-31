
use anyhow::{anyhow, Result};
use log::info;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::Command;
use std::time::Duration;
use crate::utils::adb_path;

pub struct AdbConnection {
    host: String,
    port: u16,
    time_out: u32,
    pub conn: TcpStream,
}

impl Default for AdbConnection {
    fn default() -> Self {
        Self::new("127.0.0.1", 5037, 3).unwrap()
    }
}

impl AdbConnection {
    pub fn new(host: &str, port: u16, time_out: u32) -> Result<Self> {
        let conn = Self::safe_connect(host, port, time_out)?;
        Ok(Self {
            host: host.to_string(),
            port,
            conn,
            time_out,
        })
    }

    pub fn set_timeout(&mut self, time_out: u32) -> Result<()> {
        self.conn
            .set_read_timeout(Some(Duration::from_secs(time_out as u64)))?;
        Ok(())
    }
    fn create_socket(host: &str, port: u16, time_out: u32) -> Result<TcpStream> {
        let adb_host = host;
        let adb_port = port;
        let s = TcpStream::connect((adb_host, adb_port))?;
        s.set_read_timeout(Some(Duration::from_secs(time_out as u64)))?;
        Ok(s)
    }

    fn safe_connect(host: &str, port: u16, time_out: u32) -> Result<TcpStream> {
        match Self::create_socket(host, port, time_out) {
            Ok(s) => Ok(s),
            Err(_) => {
                Command::new(adb_path()?)
                    .arg("start-server")
                    .output()
                    .expect("Failed to start adb server");
                Self::create_socket(host, port, time_out)
            }
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<usize> {
        info!(">>>>>>> Send Size: {:#?} >>>>>>>", data.len());
        let s = self.conn.write(data)?;
        Ok(s)
    }

    pub fn read(&mut self, n: usize) -> Result<Vec<u8>> {
        info!("<<<<<<< Recv Size: {:#?} <<<<<<<", n);
        let mut buffer = vec![0; n];
        self.conn.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    pub fn send_command(&mut self, cmd: &str) -> Result<()> {
        info!("Send COMMAND: <{:#?}>", cmd);
        let cmd_bytes = cmd.as_bytes();
        let length = format!("{:04x}", cmd_bytes.len());
        let mut data = Vec::with_capacity(length.len() + cmd_bytes.len());
        data.extend_from_slice(length.as_bytes());
        data.extend_from_slice(cmd_bytes);
        self.conn.write_all(&data)?;
        Ok(())
    }

    pub fn read_string(&mut self, n: usize) -> Result<String> {
        let data = self.read(n)?;
        Ok(String::from_utf8_lossy(&data).to_string())
    }

    pub fn read_string_block(&mut self) -> Result<String> {
        let length = self.read_string(4)?;
        let size = usize::from_str_radix(&length, 16)?;
        self.read_string(size)
    }

    pub fn read_until_close(&mut self) -> Result<String> {
        let mut content = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let bytes_read = self.conn.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            content.extend_from_slice(&buffer[..bytes_read]);
        }
        Ok(String::from_utf8_lossy(&content).to_string())
    }

    pub fn check_okay(&mut self) -> Result<()> {
        let data = self.read_string(4)?;
        info!("Check Okay Response Data <{:#?}>", &data);
        if data == "FAIL" {
            let error_message = self.read_string_block()?;
            Err(anyhow!(error_message))
        } else if data == "OKAY" {
            Ok(())
        } else {
            Err(anyhow!(format!("Unknown data: {}", data)))
        }
    }
}
