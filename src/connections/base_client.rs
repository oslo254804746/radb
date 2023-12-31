use crate::connections::adb_connection::AdbConnection;
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct BaseClient {
    pub host: String,
    pub port: u16,
    pub socket_timeout: u32,
}

impl Default for BaseClient {
    fn default() -> Self {
        Self::new("127.0.0.1".to_string(), 5037, 3)
    }
}

impl BaseClient {
    pub fn new<T: ToString>(host: T, port: u16, socket_timeout: u32) -> Self {
        BaseClient {
            host: host.to_string(),
            port,
            socket_timeout,
        }
    }

    pub fn connect(&mut self) -> Result<AdbConnection> {
        let conn = AdbConnection::new(&self.host, self.port, self.socket_timeout)?;
        Ok(conn)
    }

}
