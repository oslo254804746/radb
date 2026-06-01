use crate::errors::{AdbError, AdbResult};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use which::which;

#[cfg(windows)]
const ADB_EXECUTE_FILE_NAME: &'static str = "adb.exe";
#[cfg(not(windows))]
const ADB_EXECUTE_FILE_NAME: &'static str = "adb";

const ADBUTILS_ADB_PATH: &'static str = "ADBUTILS_ADB_PATH";

pub fn adb_path() -> AdbResult<PathBuf> {
    let adb_env = std::env::var(ADBUTILS_ADB_PATH);
    if adb_env.is_ok() {
        Ok(PathBuf::from(adb_env.unwrap()))
    } else {
        match which(ADB_EXECUTE_FILE_NAME) {
            Ok(path) => Ok(path),
            Err(_) => Err(AdbError::from_display("adb not found")),
        }
    }
}

pub fn get_free_port() -> AdbResult<u16> {
    let socket = TcpListener::bind("127.0.0.1:0")?;
    Ok(socket.local_addr()?.port())
}

pub fn start_adb_server() {
    start_adb_server_result().expect("Failed to start adb server");
}

pub fn start_adb_server_result() -> AdbResult<()> {
    match adb_path() {
        Err(err) => Err(err),
        Ok(path) => {
            let output = Command::new(path)
                .arg("start-server")
                .output()?;
            if output.status.success() {
                Ok(())
            } else {
                Err(AdbError::command_failed(
                    "adb start-server",
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ))
            }
        }
    }
}
