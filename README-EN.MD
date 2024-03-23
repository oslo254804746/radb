## Introduction
This project is a Rust-based library for Adb operations that is built upon openatx/adbutils.
## Usage
Adding the Dependency
```shell
cargo add radb
```
### Example Usage

#### Getting the Device List

```rust
fn main() {
    let mut adb_client = AdbConnection::default().unwrap();
    let devices = adb_client.list_devices().unwrap();
    devices.iter().for_each(|device| {
        println!("{:?}", device);
    });
}
```

#### logcat

```rust
#[cfg(test)]
mod test {
    use radb::beans::{app_info::AppInfo, file_info::FileInfo};
    use radb::client::adb_device::AdbDevice;
    use radb::utils::init_logger;

    fn setup() -> AdbDevice {
        init_logger();
        let serial = "emulator-5554";
        let device = AdbDevice::new_device_default(serial);
        device
    }

    #[test]
    fn test_logcat() {
        let mut device = setup();
        let mut mtx = Arc::new(RwLock::new(true));
        let logcat_lock = mtx.clone();
        thread::spawn(move || {
            for i in device.logcat(true, None, logcat_lock).unwrap() {
                info!("{}", i);
            }
        });
        sleep(Duration::from_secs(10));
        let mut s = mtx.write().unwrap();
        *s = false;
        info!("stop log cat");
    }

```
## Implemented Features (Corresponding to Python-AdbUtils)


|      | adb                          | device                 |
| ---- | ---------------------------- | ---------------------- |
|      | `list_device`                | `open_transport`       |
|      | `iter_device`                | `get_state`            |
|      | `get_device_by_serial`       | `shell`                |
|      | `get_device_by_transport_id` | `forward`              |
|      | `server_version`             | `forward_list`         |
|      | `server_kill`                | `reverse`              |
|      | `connect`                    | `adb_output`           |
|      | `disconnect`                 | `push`                 |
|      | device                       | `create_connection`    |
|      |                              | `tcpip`                |
|      |                              | `screenshot`           |
|      |                              | `switch_screen`        |
|      |                              | `switch_airplane_mode` |
|      |                              | `keyevent`             |
|      |                              | `switch_wifi`          |
|      |                              | `click`                |
|      |                              | `swipe`                |
|      |                              | `send_keys`            |
|      |                              | `wlan_ip`              |
|      |                              | `uninstall`            |
|      |                              | `install`              |
|      |                              | `logcat`               |
|      |                              | `...`                  |

## `Crates` Link

[crates.io](https://crates.io/crates/radb)