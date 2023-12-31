use std::net::TcpListener;
use std::path::PathBuf;
use anyhow::anyhow;
use which::which;
#[cfg(windows)]
const ADB_EXECUTE_FILE_NAME: &'static str = "adb.exe";
#[cfg(not(windows))]
const ADB_EXECUTE_FILE_NAME: &'static str = "adb";

const ADBUTILS_ADB_PATH: &'static str = "ADBUTILS_ADB_PATH";

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
    let socket = TcpListener::bind("127.0.0.1:0").unwrap();
    Ok(socket.local_addr()?.port())
}