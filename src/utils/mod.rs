use anyhow::{anyhow, Context};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use tracing::Level;
use which::which;
#[cfg(windows)]
const ADB_EXECUTE_FILE_NAME: &'static str = "adb.exe";
#[cfg(not(windows))]
const ADB_EXECUTE_FILE_NAME: &'static str = "adb";

const ADBUTILS_ADB_PATH: &'static str = "ADBUTILS_ADB_PATH";

pub fn init_logger() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(true)
        .with_thread_names(true)
        .with_thread_ids(true)
        .init();
}

pub fn adb_path() -> anyhow::Result<PathBuf> {
    let adb_env = std::env::var(ADBUTILS_ADB_PATH);
    if adb_env.is_ok() {
        Ok(PathBuf::from(adb_env.unwrap()))
    } else {
        match which(ADB_EXECUTE_FILE_NAME) {
            Ok(path) => Ok(path),
            Err(_) => Err(anyhow!("adb not found")),
        }
    }
}

pub fn get_free_port() -> anyhow::Result<u16> {
    let socket = TcpListener::bind("127.0.0.1:0")?;
    Ok(socket.local_addr()?.port())
}

pub fn start_adb_server() {
    match adb_path() {
        Err(_) => {
            panic!("Adb Path Not Found")
        }
        Ok(path) => {
            Command::new(path)
                .arg("start-server")
                .output()
                .expect("Failed to start adb server");
        }
    }
}

pub fn vec_to_string(data: &[u8]) -> anyhow::Result<String> {
    let a = String::from_utf8_lossy(&data.to_vec()).to_string();
    Ok(a)
}
